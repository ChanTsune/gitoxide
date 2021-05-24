use crate::{
    plumbing::lean::options::{self, Args, SubCommands},
    shared::lean::prepare,
};
use anyhow::Result;
use git_features::progress::DoOrDiscard;
use gitoxide_core::{self as core, OutputFormat};
use std::{
    io::{self, stderr, stdin, stdout},
    path::PathBuf,
};

pub fn main() -> Result<()> {
    let cli: Args = crate::shared::from_env();
    git_features::interrupt::init_handler(stderr());
    let thread_limit = cli.threads;
    let verbose = cli.verbose;
    match cli.subcommand {
        SubCommands::PackCreate(options::PackCreate {
            repository,
            expansion,
            tips,
        }) => {
            let (_handle, progress) = prepare(verbose, "pack-create", Some(core::pack::create::PROGRESS_RANGE));
            let has_tips = !tips.is_empty();
            let stdout = stdout();
            let stdout_lock = stdout.lock();
            #[cfg(feature = "atty")]
            if atty::is(atty::Stream::Stdout) {
                anyhow::bail!("Refusing to output pack data stream to stdout.")
            }

            core::pack::create(
                repository.unwrap_or_else(|| PathBuf::from(".")),
                tips,
                if has_tips {
                    None
                } else {
                    #[cfg(feature = "atty")]
                    if atty::is(atty::Stream::Stdin) {
                        anyhow::bail!("Refusing to read from standard input as no path is given, but it's a terminal.")
                    }
                    Some(io::BufReader::new(stdin()))
                },
                stdout_lock,
                DoOrDiscard::from(progress),
                core::pack::create::Context {
                    expansion: expansion.unwrap_or_else(|| {
                        if has_tips {
                            core::pack::create::ObjectExpansion::TreeTraversal
                        } else {
                            core::pack::create::ObjectExpansion::None
                        }
                    }),
                    thread_limit,
                },
            )
        }
        SubCommands::RemoteRefList(options::RemoteRefList { protocol, url }) => {
            let (_handle, progress) = prepare(verbose, "remote-ref-list", Some(core::remote::refs::PROGRESS_RANGE));
            core::remote::refs::list(
                protocol,
                &url,
                DoOrDiscard::from(progress),
                core::remote::refs::Context {
                    thread_limit,
                    format: OutputFormat::Human,
                    out: io::stdout(),
                },
            )
        }
        SubCommands::PackReceive(options::PackReceive {
            protocol,
            url,
            directory,
            refs_directory,
        }) => {
            let (_handle, progress) = prepare(verbose, "pack-receive", core::pack::receive::PROGRESS_RANGE);
            core::pack::receive(
                protocol,
                &url,
                directory,
                refs_directory,
                DoOrDiscard::from(progress),
                core::pack::receive::Context {
                    thread_limit,
                    format: OutputFormat::Human,
                    out: io::stdout(),
                },
            )
        }
        SubCommands::IndexFromPack(options::IndexFromPack {
            iteration_mode,
            pack_path,
            directory,
        }) => {
            use gitoxide_core::pack::index::PathOrRead;
            let (_handle, progress) = prepare(verbose, "pack-explode", core::pack::index::PROGRESS_RANGE);
            let input = if let Some(path) = pack_path {
                PathOrRead::Path(path)
            } else {
                #[cfg(feature = "atty")]
                if atty::is(atty::Stream::Stdin) {
                    anyhow::bail!("Refusing to read from standard input as no path is given, but it's a terminal.")
                }
                PathOrRead::Read(Box::new(std::io::stdin()))
            };
            core::pack::index::from_pack(
                input,
                directory,
                DoOrDiscard::from(progress),
                core::pack::index::Context {
                    thread_limit,
                    iteration_mode: iteration_mode.unwrap_or_default(),
                    format: OutputFormat::Human,
                    out: io::stdout(),
                },
            )
        }
        SubCommands::PackExplode(options::PackExplode {
            pack_path,
            sink_compress,
            object_path,
            verify,
            check,
            delete_pack,
        }) => {
            let (_handle, progress) = prepare(verbose, "pack-explode", None);
            core::pack::explode::pack_or_pack_index(
                pack_path,
                object_path,
                check.unwrap_or_default(),
                progress,
                core::pack::explode::Context {
                    thread_limit,
                    delete_pack,
                    sink_compress,
                    verify,
                },
            )
        }
        SubCommands::PackVerify(options::PackVerify {
            path,
            statistics,
            algorithm,
            decode,
            re_encode,
        }) => {
            use self::core::pack::verify;
            let (_handle, progress) = prepare(verbose, "pack-verify", None);
            core::pack::verify::pack_or_pack_index(
                path,
                progress,
                core::pack::verify::Context {
                    output_statistics: if statistics {
                        Some(core::OutputFormat::Human)
                    } else {
                        None
                    },
                    algorithm: algorithm.unwrap_or(verify::Algorithm::LessTime),
                    thread_limit,
                    mode: match (decode, re_encode) {
                        (true, false) => verify::Mode::Sha1Crc32Decode,
                        (true, true) | (false, true) => verify::Mode::Sha1Crc32DecodeEncode,
                        (false, false) => verify::Mode::Sha1Crc32,
                    },
                    out: stdout(),
                    err: stderr(),
                },
            )
            .map(|_| ())
        }
        SubCommands::CommitGraphVerify(options::CommitGraphVerify { path, statistics }) => {
            use self::core::commitgraph::verify;

            verify::graph_or_file(
                path,
                verify::Context {
                    err: stderr(),
                    out: stdout(),
                    output_statistics: if statistics {
                        Some(core::OutputFormat::Human)
                    } else {
                        None
                    },
                },
            )
            .map(|_| ())
        }
    }
}
