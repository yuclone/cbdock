use std::env;

#[derive(Debug)]
pub struct Config {
    pub protein_dir: String,
    pub ligand_dir: String,
    pub concurrency: usize,
    pub top_size: usize,
    pub root_url: Option<String>,
}

impl Config {
    pub fn parse() -> Self {
        let mut config = Config {
            protein_dir: "./蛋白质".to_string(),
            ligand_dir: "./配体".to_string(),
            concurrency: 5,
            top_size: 3,
            root_url: None,
        };

        let mut args = env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "-p" => {
                    if let Some(val) = args.next() {
                        config.protein_dir = val;
                    }
                }
                "-l" => {
                    if let Some(val) = args.next() {
                        config.ligand_dir = val;
                    }
                }
                "-n" => {
                    if let Some(val) = args.next() {
                        config.concurrency = val.parse().unwrap_or_else(|_| {
                            println!("并发数解析失败，默认使用 5");
                            5
                        });
                    }
                }
                "-a" => {
                    if let Some(val) = args.next() {
                        config.top_size = val.parse().unwrap_or_else(|_| {
                            println!("选取打分最高的数量解析失败，默认3");
                            3
                        });
                    }
                }
                "-u" => {
                    if let Some(val) = args.next() {
                        config.root_url = Some(val);
                    }
                }
                "-h" | "--help" => {
                    println!("用法: auto_tool [选项]");
                    println!("选项:");
                    println!("  -p <路径>   蛋白质文件夹路径 (默认: ./蛋白质)");
                    println!("  -l <路径>   配体文件夹路径 (默认: ./配体)");
                    println!("  -a <数字>   选取打分最高的数量");
                    println!("  -n <数字>   并发任务数量 (默认: 5)");
                    println!("  -u <URL>    指定 root_url，跳过网页自动抓取");
                    std::process::exit(0);
                }
                _ => println!("警告: 未知参数 '{}'，可使用 -h 查看帮助", arg),
            }
        }
        config
    }
}
