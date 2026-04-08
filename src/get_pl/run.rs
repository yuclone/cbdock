use super::config::Config;
use super::docking::run_docking_task;
use super::files::download_top_results;
use super::models::DockingScore;
use super::network::get_root_url;
use super::utils::get_files_with_extension;
use reqwest::Client;
use std::sync::Arc;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::parse();

    let proteins = get_files_with_extension(&config.protein_dir, "pdb");
    let ligands = get_files_with_extension(&config.ligand_dir, "mol2");

    let total_tasks = proteins.len() * ligands.len();
    println!(
        "共计 {} 个蛋白质和 {} 个配体，共 {} 个任务",
        proteins.len(),
        ligands.len(),
        total_tasks
    );

    if total_tasks == 0 {
        println!("没有需要执行的任务，程序退出。");
        return Ok(());
    }

    println!("并发任务限制数量: {}", config.concurrency);

    let client = Client::builder().cookie_store(true).build()?;
    let root_url = get_root_url(&client, config.root_url).await?;

    let semaphore = Arc::new(Semaphore::new(config.concurrency));
    let mut join_set = JoinSet::new();

    // 提交所有任务
    for p_path in &proteins {
        for l_path in &ligands {
            let p = p_path.clone();
            let l = l_path.clone();
            let sem = semaphore.clone();
            let root_url = root_url.clone();

            join_set.spawn(async move {
                match run_docking_task(p.clone(), l.clone(), sem, root_url).await {
                    Ok(res) => Ok(res),
                    Err(err) => Err((
                        p.file_name().unwrap().to_str().unwrap().to_string(),
                        l.file_name().unwrap().to_str().unwrap().to_string(),
                        err,
                    )),
                }
            });
        }
    }

    let mut all_valid_scores = process_tasks(&mut join_set).await?;
    download_top_results(&client, &mut all_valid_scores, config.top_size).await?;

    println!("\n全部任务已处理完毕, 程序退出。");
    Ok(())
}

/// 处理所有任务的执行结果并写入 Markdown 文件
async fn process_tasks(
    join_set: &mut JoinSet<
        Result<(String, String, String, Vec<DockingScore>), (String, String, String)>,
    >,
) -> Result<Vec<DockingScore>, Box<dyn std::error::Error>> {
    let mut result_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("result.md")
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
