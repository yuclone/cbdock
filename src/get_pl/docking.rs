use super::models::DockingScore;
use super::network::{create_file_part, fetch_progress_text};
use super::regex_utils::*;
use reqwest::{Client, multipart};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Semaphore;
use tokio::time::{Duration, sleep};

pub async fn run_docking_task(
    p_path: PathBuf,
    l_path: PathBuf,
    semaphore: Arc<Semaphore>,
    root_url: String,
) -> Result<(String, String, String, Vec<DockingScore>), String> {
    let p_name = p_path.file_name().unwrap().to_str().unwrap().to_string();
    let l_name = l_path.file_name().unwrap().to_str().unwrap().to_string();

    let mut last_err = String::new();
    for i in 1..=3 {
        match run_docking_task_single(p_path.clone(), l_path.clone(), semaphore.clone(), root_url.clone()).await {
            Ok(res) => return Ok(res),
            Err(e) => {
                last_err = e;
                if i < 3 {
                    println!(
                        "[{} * {}] 对接失败, 正在进行第 {} 次重试... 错误: {}",
                        p_name, l_name, i, last_err
                    );
                    sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }
    Err(format!("任务在尝试 3 次后仍失败: {}", last_err))
}

async fn run_docking_task_single(
    p_path: PathBuf,
    l_path: PathBuf,
    semaphore: Arc<Semaphore>,
    root_url: String,
) -> Result<(String, String, String, Vec<DockingScore>), String> {
    let p_name = p_path.file_name().unwrap().to_str().unwrap().to_string();
    let l_name = l_path.file_name().unwrap().to_str().unwrap().to_string();

    // 获取信号量许可，限制并发
    let _permit = semaphore.acquire().await.map_err(|e| e.to_string())?;

    //为每一个对接申请一个新的客户端，否则网站会使用cookie存储user身份信息，同一个user同时只能运行一个任务，不能真正并发
    let client = Client::builder().cookie_store(true).build().unwrap();

    //println!("{}", root_html);
    println!("[{} *{}] 开始对接...", p_name, l_name);
    let base_url = root_url.clone() + &"/php".to_string();

    //let base_url = "http://cao.labshare.cn:11080/cb-dock2/php";
    // 1. 获取 temp_dir 和 userName
    let home_html = fetch_progress_text(&client, &format!("{}/blinddock.php", base_url)).await?;

    let temp_dir = get_regex_temp()
        .captures(&home_html)
        .ok_or("找不到 temp_dir")?[1]
        .to_string();
    let user_name = get_regex_user()
        .captures(&home_html)
        .ok_or("找不到 userName")?[1]
        .to_string();

    // 2. 上传蛋白质
    let p_form = multipart::Form::new()
        .part("protein_file", create_file_part(&p_path).await?)
        .text("temp_dir", temp_dir.clone());
    client
        .post(format!("{}/upload.php", base_url))
        .multipart(p_form)
        .send()
        .await
        .map_err(|e| format!("蛋白质上传失败: {}", e))?;

    // 3. 上传配体
    let l_form = multipart::Form::new()
        .part("ligand_file", create_file_part(&l_path).await?)
        .text("temp_dir", temp_dir.clone());
    client
        .post(format!("{}/upload.php", base_url))
        .multipart(l_form)
        .send()
        .await
        .map_err(|e| format!("配体上传失败: {}", e))?;

    // 4. 提交对接任务
    let submit_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
        .to_string();
    let task_form = multipart::Form::new()
        .text("temp_dir", temp_dir.clone())
        .text("submit_time", submit_time.clone())
        .text("up_pocket", "5")
        .text("custom_pocket", "null")
        .text("check_Tligand_upload", "0")
        .text("check_Tprotein_upload", "0")
        .text("email", "null")
        .text("keep_hetatms", ",");

    client
        .post(format!("{}/auto_blinddock.php", base_url))
        .multipart(task_form)
        .send()
        .await
        .map_err(|e| format!("任务提交失败: {}", e))?;

    // 5. 轮询等待结果
    let prog_url = format!(
        "{}/auto_blinddock_progress.php?user=guest&id={}&token={}",
        base_url, user_name, submit_time
    );

    loop {
        let prog_html = fetch_progress_text(&client, &prog_url).await?;

        if let Some(caps) = get_regex_error().captures(&prog_html) {
            let err_msg = caps.get(1).unwrap().as_str();
            if !err_msg.is_empty() {
                return Err(format!("服务器计算报错: {}", err_msg));
            }
        }

        if let Some(caps) = get_regex_percent().captures(&prog_html) {
            let percent: f64 = caps.get(1).unwrap().as_str().parse().unwrap_or(0.0);
            if percent >= 1.0 {
                let result_link = format!(
                    "{}/show_auto_blinddock.php?user=guest&id={}&token={}",
                    base_url, user_name, submit_time
                );

                // --- 任务完成，开始提取分数和最终链接 ---
                let final_html = fetch_progress_text(&client, &result_link).await?;

                let current_job_dir_raw = get_regex_job_dir()
                    .captures(&final_html)
                    .ok_or("页面中找不到 current_jobDir 变量")?[1]
                    .to_string();

                // 处理 JS 中可能的 "\/" 转义，并去掉相对路径前缀
                let unescaped_dir = current_job_dir_raw.replace("\\/", "/");
                let clean_dir = unescaped_dir.trim_start_matches("./../");

                let conf_url = format!("{}/{}/conf_after_dock.txt", root_url, clean_dir);

                let conf_text = fetch_progress_text(&client, &conf_url).await?;
                let mut scores = Vec::new();

                let p_name_clean = p_name.trim_end_matches(".pdb");
                let l_name_clean = l_name.trim_end_matches(".mol2");

                for line in conf_text.lines().skip(1) {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 9 {
                        let id = parts[0];
                        let score_str = parts[8];

                        if let Ok(score_val) = score_str.parse::<f64>() {
                            if score_val < 0.0 {
                                let download_link = format!(
                                    "{}/{}/{}:{}_out_{}.{}.complex.pdb",
                                    root_url, clean_dir, p_name_clean, l_name_clean, id, score_str
                                );

                                scores.push(DockingScore {
                                    protein: p_name.clone(),
                                    ligand: l_name.clone(),
                                    score: score_val,
                                    download_link,
                                });
                            }
                        }
                    }
                }

                return Ok((p_name, l_name, result_link, scores));
            }
        }

        sleep(Duration::from_secs(5)).await;
    }
}
