use anyhow::Result;

pub struct Options {
    // -c
    pub concurrency: usize,
    // -w
    pub write: bool,
}

impl Options {
    pub fn new() -> Result<Self> {
        let args: Vec<String> = std::env::args().collect();

        let mut concurrency: usize = 10;
        let mut write = false;

        for (i, arg) in args.iter().enumerate() {
            if arg == "-c" {
                if let Some(x) = args.get(i + 1) {
                    concurrency = x.parse()?;
                }
            }
            if arg == "-w" {
                write = true;
            }
        }

        return Ok(Options { concurrency, write });
    }
}
