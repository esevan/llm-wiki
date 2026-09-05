fn main() {
    let mut args = std::env::args().skip(1);
    if args.next().as_deref() == Some("--mcp") {
        let connection = match (
            args.next().as_deref(),
            args.next(),
            std::env::var("LLM_WIKI_CONNECTION_ID").ok(),
        ) {
            (Some("--connection"), Some(value), _) => value,
            (_, _, Some(value)) if !value.trim().is_empty() => value,
            _ => {
                eprintln!("Set LLM_WIKI_CONNECTION_ID or use --connection <id>");
                std::process::exit(2);
            }
        };
        let runtime = tokio::runtime::Runtime::new().expect("Could not start MCP runtime");
        if let Err(error) = runtime.block_on(llm_wiki_desktop::run_mcp(connection)) {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return;
    }
    llm_wiki_desktop::run();
}
