use std::error::Error;
use std::str;
use std::{path::PathBuf, process::Command};

use clap::Parser;
use jack::analyzer::AnalyzerErr;
use jack::codegen::ClassWriter;
use jack::macr::lex_and_macronize;
use jack::parser::{parse, ParserErr};
use klex::Loc;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// The file to compile
    #[arg()]
    file: PathBuf,

    /// The path to the Jasmin jar file
    #[arg(short, long)]
    jasmin: PathBuf,

    /// What file to compile to
    #[arg(short, long)]
    out: Option<PathBuf>,

    /// Emit the post-macro final source?
    #[arg(short, action)]
    macro_emit: bool,

    /// Print debug info?
    #[arg(short, long, action)]
    debug: bool,
}

fn main() {
    let args = Args::parse();
    let out = args.out.unwrap_or_else(|| {
        let mut path = args.file.clone();
        path.set_extension("class");
        path
    });
    let jasmin_file = {
        let mut path = args.file.clone();
        path.set_extension("j");
        path
    };

    let jack_source = std::fs::read_to_string(args.file.clone()).expect("err reading file");
    let (tokens, _, jack_source) = lex_and_macronize(jack_source, 0, args.debug).expect("error lexing");
    if args.macro_emit {
        let out = {
            let mut path = args.file.clone();
            path.set_extension("post-macro.jack");
            path
        };
        std::fs::write(out, tokens.iter().map(|rt| rt.inner.spelling()).collect::<String>()).expect("cannot write");
    }
    let class_name = out.file_stem().unwrap().to_str().unwrap();

    let mut ast = parse(tokens.into_iter().map(|t| Ok(t)), 0)
        .map_err(|e| report_parser_err(e, &jack_source))
        .expect("parsing err");
    let analyzer = ast.analyze(args.debug)
        .map_err(|e| report_analyzer_err(e, &jack_source))
        .expect("analyzer err");
    let mut class = ClassWriter::new(
        args.file.file_name().unwrap().to_str().unwrap().into(),
        class_name.into(),
        "java/lang/Object".into(),
    );
    ast.code_gen(&mut class, analyzer.max_stack_size, analyzer.max_vars_count + 1)
        .map_err(|e| println!("{e}"))
        .expect("code gen err");

    std::fs::write(&jasmin_file, class.write()).expect("error writing assmbly!");
    let jasmin_cmd_out = Command::new("java")
        .arg("-jar")
        .arg(args.jasmin)
        .arg(jasmin_file)
        .output()
        .expect("error executing jasmin!");
    println!("{}", str::from_utf8(&jasmin_cmd_out.stdout).unwrap());
    println!("{}", str::from_utf8(&jasmin_cmd_out.stderr).unwrap());
}

fn report_parser_err(e: ParserErr, src: &str) {
    println!("{e}");
    if let Some(loc) = e.loc() {
        print_err_loc(loc, src);
    }
}

fn report_analyzer_err(e: AnalyzerErr, src: &str) {
    println!("{e}");
    if let Some(loc) = e.loc() {
        print_err_loc(loc, src);
    }
}

fn print_err_loc(loc: Loc, src: &str) {
    let line_above = if loc.row < 2 {
        String::new()
    } else {
        src.lines().nth(loc.row - 2).map(String::from).unwrap_or_else(|| String::new())
    };
    if let Some(line) = src.lines().nth(loc.row - 1) {
        println!("     | {line_above}");
        println!("{: >4} | {line}", loc.row);
        print!("{}", "-".repeat(loc.col + 6));
        println!("^");
    }
}

