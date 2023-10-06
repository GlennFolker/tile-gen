#![feature(path_file_prefix)]

use clap::{
    error::{
        Error as CliError,
        ErrorKind as CliErrorKind,
    },
    Parser, Subcommand,
};
use hashbrown::HashMap;
use image::{
    io::Reader,
    ImageFormat, ImageError,
    Rgba, RgbaImage,
};
use rayon::{
    prelude::*,
    ThreadPoolBuilder, ThreadPoolBuildError,
};
use thiserror::Error;
use std::{
    io::Error as IoError,
    ffi::{
        OsStr, OsString,
    },
    path::Path,
    process::ExitCode,
};

#[derive(Error, Debug)]
enum TilegenError {
    #[error("image dimension ({0}, {1}) is indivisible by 4")]
    IndivisibleBy4(u32, u32),
    #[error("image dimension ({0}, {1}) is not square")]
    NotSquare(u32, u32),
    #[error("{0}")]
    ImageError(#[from] ImageError),
    #[error("{0}")]
    IoError(#[from] IoError),
    #[error("{0}")]
    CliError(#[from] CliError),
    #[error("{0}")]
    ThreadPoolError(#[from] ThreadPoolBuildError),
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Process input files
    Proc {
        /// Specify the amount of threads to work
        #[arg(short, long)]
        jobs: Option<u8>,
        /// Padding amount, in pixels
        #[arg(short, long, default_value_t = 0)]
        pad: u32,
        /// Bleeding amount, in pixels less or equal to padding amount
        #[arg(short, long, default_value_t = 0)]
        bleed: u32,
        /// The .png files
        #[arg(required(true))]
        files: Vec<OsString>,
    },
    /// Print out bitmask index mapping
    Mapping,
}

fn main() -> ExitCode {
    const LAYOUT_WIDTH: u32 = 384;
    const LAYOUT_HEIGHT: u32 = 128;

    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => return match e.kind() {
            CliErrorKind::DisplayHelp |
            CliErrorKind::DisplayHelpOnMissingArgumentOrSubcommand |
            CliErrorKind::DisplayVersion => {
                println!("{e}");
                ExitCode::SUCCESS
            },
            _ => {
                eprintln!("{e}");
                ExitCode::FAILURE
            },
        },
    };

    match cli.cmd {
        Cmd::Proc { jobs, pad, bleed, files, } => {
            if bleed > pad {
                eprintln!("--bleed may not be greater than --pad");
                return ExitCode::FAILURE;
            }

            // Initialize global thread pool.
            if let Err(e) = {
                let mut builder = ThreadPoolBuilder::new();
                if let Some(jobs) = jobs {
                    builder = builder.num_threads(jobs as usize);
                }

                builder.build_global()
            } {
                eprintln!("{e}");
                return ExitCode::FAILURE;
            }

            // Load the layout image.
            let layout = match image::load_from_memory_with_format(
                include_bytes!("layout.png"),
                ImageFormat::Png,
            ) {
                Ok(layout) => layout,
                Err(e) => {
                    eprintln!("{e}");
                    return ExitCode::FAILURE;
                },
            }.into_rgba8();

            #[cfg(debug_assertions)]
            {
                let (width, height) = layout.dimensions();
                assert_eq!(width, LAYOUT_WIDTH, "layout's width must be {LAYOUT_WIDTH}");
                assert_eq!(height, LAYOUT_HEIGHT, "layout's height must be {LAYOUT_HEIGHT}");
            }

            let error = files
                .into_par_iter()
                .map::<_, Result<(), (String, TilegenError)>>(|file| {
                    #[inline]
                    fn err(e: impl Into<TilegenError>, file: &OsStr) -> (String, TilegenError) {
                        (file.to_string_lossy().into_owned(), e.into())
                    }

                    let image = {
                        let mut reader = Reader::open(Path::new(&file)).map_err(|e| err(e, &file))?;
                        reader.set_format(ImageFormat::Png);
                        reader.decode().map_err(|e| err(e, &file))?
                    }.into_rgba8();

                    let (width, height) = image.dimensions();
                    if width % 4 != 0 || height % 4 != 0 { return Err(err(TilegenError::IndivisibleBy4(width, height), &file)); }
                    if width != height { return Err(err(TilegenError::NotSquare(width, height), &file)); }

                    let cell_size = width / 4;
                    let padded_size = cell_size + 2 * pad;

                    let mut palettes = HashMap::<Rgba<u8>, (u32, u32)>::default();
                    for x in 0..4 {
                        for y in 0..4 {
                            palettes.insert(*layout.get_pixel(
                                x * LAYOUT_WIDTH / 12,
                                y * LAYOUT_HEIGHT / 4,
                            ), (
                                x * width / 4,
                                y * height / 4,
                            ));
                        }
                    }

                    let out_width = (width / 4 + 2 * pad) * 12;
                    let out_height = (height / 4 + 2 * pad) * 4;

                    let mut out = RgbaImage::new(out_width, out_height);
                    for cx in 0..12 {
                        for cy in 0..4 {
                            for rx in 0..cell_size {
                                for ry in 0..cell_size {
                                    let Some(&(sx, sy)) = palettes.get(layout.get_pixel(
                                        (cx * cell_size + rx) * LAYOUT_WIDTH / (width * 3),
                                        (cy * cell_size + ry) * LAYOUT_HEIGHT / height,
                                    )) else { continue };

                                    out.put_pixel(
                                        pad + cx * padded_size + rx,
                                        pad + cy * padded_size + ry,
                                        *image.get_pixel(
                                            sx + rx,
                                            sy + ry,
                                        ),
                                    );
                                }
                            }
                        }
                    }

                    for b in 0..bleed {
                        for cx in 0..12 {
                            for cy in 0..4 {
                                for x in (pad - b)..(pad + cell_size + b) {
                                    out.put_pixel(
                                        cx * padded_size + x,
                                        cy * padded_size + pad - b - 1,
                                        *out.get_pixel(
                                            cx * padded_size + x,
                                            cy * padded_size + pad - b,
                                        ),
                                    );

                                    out.put_pixel(
                                        cx * padded_size + x,
                                        (cy + 1) * padded_size - pad + b,
                                        *out.get_pixel(
                                            cx * padded_size + x,
                                            (cy + 1) * padded_size - pad + b - 1,
                                        ),
                                    );
                                }

                                for y in (pad - b)..(pad + cell_size + b) {
                                    out.put_pixel(
                                        cx * padded_size + pad - b - 1,
                                        cy * padded_size + y,
                                        *out.get_pixel(
                                            cx * padded_size + pad - b,
                                            cy * padded_size + y,
                                        ),
                                    );

                                    out.put_pixel(
                                        (cx + 1) * padded_size - pad + b,
                                        cy * padded_size + y,
                                        *out.get_pixel(
                                            (cx + 1) * padded_size - pad + b - 1,
                                            cy * padded_size + y,
                                        ),
                                    );
                                }

                                out.put_pixel(
                                    cx * padded_size + pad - b - 1,
                                    cy * padded_size + pad - b - 1,
                                    *out.get_pixel(
                                        cx * padded_size + pad - b,
                                        cy * padded_size + pad - b,
                                    ),
                                );

                                out.put_pixel(
                                    (cx + 1) * padded_size - pad + b,
                                    cy * padded_size + pad - b - 1,
                                    *out.get_pixel(
                                        (cx + 1) * padded_size - pad + b - 1,
                                        cy * padded_size + pad - b,
                                    ),
                                );

                                out.put_pixel(
                                    (cx + 1) * padded_size - pad + b,
                                    (cy + 1) * padded_size - pad + b,
                                    *out.get_pixel(
                                        (cx + 1) * padded_size - pad + b - 1,
                                        (cy + 1) * padded_size - pad + b - 1,
                                    ),
                                );

                                out.put_pixel(
                                    cx * padded_size + pad - b - 1,
                                    (cy + 1) * padded_size - pad + b,
                                    *out.get_pixel(
                                        cx * padded_size + pad - b,
                                        (cy + 1) * padded_size - pad + b - 1,
                                    ),
                                );
                            }
                        }
                    }

                    out.save_with_format(
                        Path::new(&file).with_file_name({
                            let mut name = OsString::from(Path::new(&file).file_prefix().unwrap());
                            name.push("-tiled.png");
                            name
                        }),
                        ImageFormat::Png,
                    ).map_err(|e| err(e, &file))?;

                    Ok(())
                })
                .fold(
                    || String::new(),
                    |mut message, result| {
                        if let Err((file, e)) = result {
                            message.push_str("Error processing file '");
                            message.push_str(&file);
                            message.push_str("': ");
                            message.push_str(&e.to_string());
                        }
                        message
                    },
                )
                .reduce(
                    || String::new(),
                    |mut a, b| {
                        a.push_str(&b);
                        a
                    },
                );

            if !error.is_empty() {
                eprintln!("{error}");
                return ExitCode::FAILURE;
            }
        },
        Cmd::Mapping => println!(
"39, 36, 39, 36, 27, 16, 27, 24, 39, 36, 39, 36, 27, 16, 27, 24,
38, 37, 38, 37, 17, 41, 17, 43, 38, 37, 38, 37, 26, 21, 26, 25,
39, 36, 39, 36, 27, 16, 27, 24, 39, 36, 39, 36, 27, 16, 27, 24,
38, 37, 38, 37, 17, 41, 17, 43, 38, 37, 38, 37, 26, 21, 26, 25,
 3,  4,  3,  4, 15, 40, 15, 20,  3,  4,  3,  4, 15, 40, 15, 20,
 5, 28,  5, 28, 29, 10, 29, 23,  5, 28,  5, 28, 31, 11, 31, 32,
 3,  4,  3,  4, 15, 40, 15, 20,  3,  4,  3,  4, 15, 40, 15, 20,
 2, 30,  2, 30,  9, 46,  9, 22,  2, 30,  2, 30, 14, 44, 14,  6,
39, 36, 39, 36, 27, 16, 27, 24, 39, 36, 39, 36, 27, 16, 27, 24,
38, 37, 38, 37, 17, 41, 17, 43, 38, 37, 38, 37, 26, 21, 26, 25,
39, 36, 39, 36, 27, 16, 27, 24, 39, 36, 39, 36, 27, 16, 27, 24,
38, 37, 38, 37, 17, 41, 17, 43, 38, 37, 38, 37, 26, 21, 26, 25,
 3,  0,  3,  0, 15, 42, 15, 12,  3,  0,  3,  0, 15, 42, 15, 12,
 5,  8,  5,  8, 29, 35, 29, 33,  5,  8,  5,  8, 31, 34, 31,  7,
 3,  0,  3,  0, 15, 42, 15, 12,  3,  0,  3,  0, 15, 42, 15, 12,
 2,  1,  2,  1,  9, 45,  9, 19,  2,  1,  2,  1, 14, 18, 14, 13,"
        ),
    }

    return ExitCode::SUCCESS;
}
