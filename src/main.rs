mod get_pl;

fn main() {
    get_pl::main().unwrap_or_else(|e| println!("程序运行出错: {}", e));
}
