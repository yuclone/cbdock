use reqwest::{Client, multipart};
use std::path::Path;
use tokio::fs::File;
use tokio::time::{Duration, sleep};
use tokio_util::codec::{BytesCodec, FramedRead};

pub async fn create_file_part(path: &Path) -> Result<multipart::Part, String> {
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();
    let file = File::open(path)
        .await
        .map_err(|e| format!("打开文件失败: {}", e))?;
    let stream = FramedRead::new(file, BytesCodec::new());
    let body = reqwest::Body::wrap_stream(stream);
    Ok(multipart::Part::stream(body).file_name(filename))
}

pub async fn fetch_progress_text(client: &Client, url: &str) -> Result<String, String> {
    let mut retry_count = 0;
    loop {
        match client.get(url).send().await {
            Ok(resp) => match resp.text().await {
                Ok(text) => return Ok(text),
                Err(e) => {
                    if retry_count >= 5 {
                        return Err(format!("数据接收异常中止: {}", e));
                    }
                }
            },
            Err(e) => {
                if retry_count >= 5 {
                    return Err(format!("网络请求彻底失败: {}", e));
                }
            }
        }
        retry_count += 1;
        sleep(Duration::from_secs(3)).await;
    }
}

pub async fn get_root_url(
    client: &Client,
    config_url: Option<String>,
) -> Result<String, Box<dyn std::error::Error>> {
    if let Some(url) = config_url {
        println!("使用指定的 root_url: {}", url);
        return Ok(url);
    }

    println!("正在从服务器获取 root_url...");
    let root_html = fetch_progress_text(client, "https://cadd.labshare.cn/cb-dock2/").await?;

    let root_urls: Vec<&str> = super::regex_utils::get_regex_url()
        .find_iter(&root_html)
        .map(|mat| mat.as_str())
        .collect();

    if root_urls.is_empty() {
        return Err("无法从主页匹配到 root_url".into());
    }

    let fetched_url = root_urls[0].to_string();
    println!("自动获取到 root_url: {}", fetched_url);
    Ok(fetched_url)
}

pub async fn download_file(client: &Client, url: &str, dest: &Path) -> Result<(), String> {
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("服务器返回错误状态码: {}", resp.status()));
    }
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("读取数据失败: {}", e))?;
    tokio::fs::write(dest, bytes)
        .await
        .map_err(|e| format!("文件写入失败: {}", e))?;
    Ok(())
}
