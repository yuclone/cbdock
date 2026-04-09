use super::docking::run_docking_task;
use super::io::{download_top_results, process_tasks};
use super::models::*;
use super::network::get_root_url;
use super::utils::get_files_with_extension;
use reqwest::Client;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::parse();

    let proteins = get_files_with_extension(&config.protein_dir, "pdb");
    let ligands = get_files_with_extension(&config.ligand_dir, "mol2");

    let total_tasks = proteins.len() * ligands.len();
    println!(
        "共计 {} 个蛋白质和 {} 个配体，共 {} 个任务\n将选取并下载打分top {} 的对接产物",
        proteins.len(),
        ligands.len(),
        total_tasks,
        &config.top_size
    );
    println!("并发任务限制数量: {}", config.concurrency);

    if total_tasks == 0 {
        println!("没有需要执行的任务，程序退出。");
        return Ok(());
    }

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
