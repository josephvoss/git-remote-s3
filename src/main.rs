extern crate clap;
use clap::{Arg, App, SubCommand, crate_authors, crate_version};
use anyhow::{Context, Result};

fn main() -> Result<()> {
    let matches = App::new("git-remote-s3")
                          .version(crate_version!())
                          .author(crate_authors!(","))
                          .arg(Arg::with_name("config")
                               .short("c")
                               .long("config")
                               .value_name("FILE")
                               .help("Sets a custom config file")
                               .takes_value(true)
                               .env("GIT_S3_CONFIG"))
                          .arg(Arg::with_name("v")
                               .short("v")
                               .multiple(true)
                               .help("Sets the level of verbosity"))
                          .arg(Arg::with_name("remote-name")
                               .help("Name of remote repository")
                               .required(true)
                               .index(1))
                          .arg(Arg::with_name("remote-bucket")
                               .help("URL to remote S3 bucket")
                               .required(true)
                               .index(2))
/* I don't think we actually need subcommands
                          .subcommand(SubCommand::with_name("capabilities")
                                      .about("Lists supported features"))
                          .subcommand(SubCommand::with_name("fetch")
                                      .about("Get remote refs and download referenced objects"))
                          .subcommand(SubCommand::with_name("push")
                                      .about("Change remote refs and push objects up to them"))
*/
                          .get_matches();

    // Load vars
    let config = matches.value_of("config").unwrap_or("~/.git-remote-s3.config");
    let remote = matches.value_of("remote-name")
        .with_context(|| format!("No remote-name argument specifed!"))?;
    let bucket = matches.value_of("remote-bucket")
        .with_context(|| format!("No remote-bucket argument specifed!"))?;
    //println!("Value for config: {}", config);

    // Set logging level
    match matches.occurrences_of("v") {
        0 => println!("No verbose info"),
        1 => println!("Some verbose info"),
        2 => println!("Tons of verbose info"),
        3 | _ => println!("Don't be crazy"),
    }

    // Build git_s3 object

    // Lock stdin

    // Loop over commands on stdin, do work

    // Return
    Ok(())
}
