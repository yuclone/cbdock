use super::models::DockingScore;
use super::network::download_file;
use chrono::Local;
use reqwest::Client;
use std::path::Path;
use tokio::fs::{self, File, OpenOptions};
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};

/// 处理所有任务的执行结果并写入 Markdown 文件
pub async fn process_tasks(
    join_set: &mut tokio::task::JoinSet<
        Result<(String, String, String, Vec<DockingScore>), (String, String, String)>,
    >,
) -> Result<Vec<DockingScore>, Box<dyn std::error::Error>> {
    let mut result_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("result.md")
        .await?;
    let local_time = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let t1 = format!("<h1><center>{}</center></h1>  \n\n", local_time);
    let t2 = "> 请注意结果链接有效期, 作者测试时发现服务器只会保存大约一天  \n\n";
    let t3 = "| 蛋白 * 配体 | 链接 | 打分 | 打分 | 打分 | 打分 | 打分 |  \n";
    let t4 = "| :---: | :---: | :---: | :---: | :---: | :---: | :---: |  \n";
    result_file
        .write_all((t1 + t2 + t3 + t4).as_bytes())
        .await?;
    let mut all_valid_scores: Vec<DockingScore> = Vec::new();

    while let Some(res) = join_set.join_next().await {
        match res {
            Ok(Ok((p_name, l_name, link, mut task_scores))) => {
                let scores_str = task_scores
                    .iter()
                    .map(|s| format!("{:.1}", s.score))
                    .collect::<Vec<_>>()
                    .join(", ");

                let msg = format!(
                    "[{} *{}]对接完成, [结果链接]({})  {}",
                    p_name, l_name, link, scores_str
                );

                println!(
                    "[{} *{}]对接完成, [\x1b]8;;{}\x1b\\结果链接\x1b]8;;\x1b\\], {}",
                    p_name, l_name, link, scores_str
                );
                result_file
                    .write_all(format!("{}  \n", msg).as_bytes())
                    .await?;

                all_valid_scores.append(&mut task_scores);
            }
            Ok(Err(err)) => {
                let msg = format!("❌ [{} *{}]对接失败: {}", err.0, err.1, err.2);
                println!("{}", msg);
                result_file
                    .write_all(format!("{}  \n", msg).as_bytes())
                    .await?;
            }
            Err(e) => {
                println!("⚠️ 运行时崩溃: {}", e);
            }
        }
    }
    result_file.flush().await?;
    Ok(all_valid_scores)
}

/// 提取 Top 结果并下载产物（全异步版）
pub async fn download_top_results(
    client: &Client,
    all_valid_scores: &mut Vec<DockingScore>,
    top_size: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    if all_valid_scores.is_empty() {
        println!("\n⚠️ 所有的对接结果中都没有找到有效打分。");
        return Ok(());
    }

    println!("\n========= 🏆 全局结合得分 Top =========");

    // 使用 tokio 的 OpenOptions
    let mut result_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("result.md")
        .await?;

    result_file
        .write_all("\n\n### 全局结合得分 Top \n| TOP | 蛋白 | 配体 | 得分 | 链接 | \n | :---: | :---: | :---: | :---: | :---: | \n".as_bytes())
        .await?;

    // 排序逻辑保持不变（同步计算）
    all_valid_scores.sort_by(|a, b| a.score.total_cmp(&b.score));
    let top: Vec<_> = all_valid_scores.iter().take(top_size).collect();

    let download_dir = Path::new("./对接结果");
    if !download_dir.exists() {
        // 使用异步创建目录
        fs::create_dir_all(download_dir).await?;
    }

    for (i, item) in top.iter().enumerate() {
        let rank = i + 1;
        let log_msg = format!(
            "TOP {}: 蛋白[{}], 配体[{}], 得分: {}, [\x1b]8;;{}\x1b\\下载链接\x1b]8;;\x1b\\]",
            rank, item.protein, item.ligand, item.score, item.download_link
        );
        println!("{}", log_msg);

        let md_msg = format!(
            "| **TOP {}** | 蛋白: `{}` | 配体: `{}` | 得分: **{}** | 🔽 [下载产物]({}) |",
            rank, item.protein, item.ligand, item.score, item.download_link
        );

        result_file
            .write_all(format!("{}  \n", md_msg).as_bytes())
            .await?;

        let safe_p = item.protein.trim_end_matches(".pdb");
        let safe_l = item.ligand.trim_end_matches(".mol2");
        let file_name = format!("TOP{}_{}_{}.pdb", rank, safe_p, safe_l);
        let dest_path = download_dir.join(&file_name);

        println!("  ⬇️ 正在下载产物文件: {} ...", file_name);
        match download_file(client, &item.download_link, &dest_path).await {
            Ok(_) => {
                println!("  ✅ 下载成功: {:?}", dest_path);
                // 这里现在需要 await，因为函数变异步了
                process_file_content(&dest_path).await?;
            }
            Err(e) => println!("  ❌ 下载失败: {}", e),
        }
    }

    result_file.flush().await?;
    Ok(())
}

/// 异步处理文件内容
pub async fn process_file_content(path: &Path) -> io::Result<()> {
    // 1. 异步打开原始文件
    let file = File::open(path).await?;
    let mut reader = BufReader::new(file).lines(); // 注意这里异步的 lines()

    // 2. 创建临时文件
    let temp_path = path.with_extension("tmp");
    let mut temp_file = File::create(&temp_path).await?;

    // 3. 异步逐行处理
    while let Some(line) = reader.next_line().await? {
        if !line.starts_with(['E', 'C', 'M']) {
            // 使用 write_all 替代 writeln! 宏在异步文件上的直接操作
            // 或者使用 format_args!，但通常 write_all 配合 \n 比较直接
            temp_file
                .write_all(format!("{}\n", line).as_bytes())
                .await?;
        }
    }

    // 4. 确保写入并重命名
    temp_file.flush().await?;
    fs::rename(temp_path, path).await?;

    Ok(())
}
