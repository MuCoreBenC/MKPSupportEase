/* 把 payload.zip 压成 payload.xz（构建期用，不进最终安装器）。
 *
 * 用法：pack <输入> <输出>
 *
 * 为什么单独一个 bin：压缩发生在**打包时**，安装器只需要解压；
 * 把压缩代码留在主程序里会让 exe 多背一份用不到的字典表。
 */

use std::fs::File;
use std::io::{BufReader, BufWriter};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        eprintln!("用法：pack <输入.zip> <输出.xz>");
        std::process::exit(2);
    }
    let input = File::open(&args[0]).expect("打不开输入文件");
    let output = File::create(&args[1]).expect("建不了输出文件");
    lzma_rs::xz_compress(
        &mut BufReader::new(input),
        &mut BufWriter::new(output),
    )
    .expect("xz 压缩失败");
}
