//use color_print::cformat;
static STATUS_OK: &str      = "[   \x1b[1;92m OK \x1b[0m   ]";
static STATUS_FAILED: &str  = "[ \x1b[1;91m FAILED \x1b[0m ]";
static VERSION: &str = env!("CARGO_PKG_VERSION");
const DEPS_DEBIAN: &[u8] = include_bytes!("scripts/deps_debian");
const DEPS_ARCH: &[u8] = include_bytes!("scripts/deps_arch");
const COMPILE_IW1X: &[u8] = include_bytes!("scripts/compile_iw1x");

use std::{env, io};
use std::fs::{create_dir_all, File, read_dir, copy, remove_file, write, OpenOptions};
use std::os::unix::fs::OpenOptionsExt;
use io::{Read, Write};
use std::path::Path;
use reqwest::blocking::get;
use indicatif::{ProgressBar, ProgressStyle};
use md5::Context;
use std::time::Duration;
use bzip2::read::BzDecoder;
use tar::Archive;
use sysinfo::System;
use std::process::{Command, Stdio};

//fn create_symlink(src: &Path, dst: &Path) -> io::Result<()> {
//    return std::os::unix::fs::symlink(src, dst);
//}

fn main() -> io::Result<()> {

    println!("Free Palestine 🍉️ 🇵🇸️ \n\n");
    println!("\x1b[1;32mO\x1b[0m\x1b[1;95mrbix\x1b[0m v{} \n", &VERSION);

    println!("\nIt is recommended to update your system before running Orbix.");
    println!("If you didn't do that, you can quit Orbix right now by pressing Ctrl + C.");
    println!("If your system is upto-date, press ENTER\n");
    io::stdin().read_line(&mut String::new())?;

    let args: Vec<String> = env::args().collect();

    let target = if args.len() == 2
    {
        args[1].clone()
    }
    else if args.len() == 1
    {
        println!("\x1b[1;94mEnter the target directory. It should be an absolute path and empty directory!\x1b[0m");
        print!("Target Directory: ");
        io::stdout().flush()?;
        let mut target = String::new();
        io::stdin().read_line(&mut target)?;
        let target = target.trim().to_string();

        target
    }
    else
    {
        eprintln!("Usage: {} target", args[0]);
        std::process::exit(1);
    };

    let target_path = Path::new(&target);

    if !target_path.exists() || !target_path.is_dir() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "Target is not a directory"));
    }
    {
        match read_dir(target_path) {
            Ok(entries) => {
                // Check if there are any entries in the directory
                for _ in entries {
                    return Err(io::Error::new(io::ErrorKind::Other, "Target directory is not empty"));
                }
            }
            Err(_) => {
                eprintln!("Failed to read the directory: {}", target_path.to_str().unwrap());
                return Err(io::Error::new(io::ErrorKind::Other, "Failed to read the directory"));
            }
        }
    }
    let e = create_dirs(target_path);
    e.expect("I couldn't create necessary directories");

    println!("\n\x1b[1;94mOkay, I will check and download server files if needed.");
    println!("Downloading is done only once if files remain intact.\x1b[0m");

    println!("");

    print!("Starting in \x1b[1;93m3\x1b[0m..."); io::stdout().flush()?;
    std::thread::sleep(Duration::from_secs(1));
    print!("\rStarting in \x1b[1;93m2\x1b[0m..."); io::stdout().flush()?;
    std::thread::sleep(Duration::from_secs(1));
    print!("\rStarting in \x1b[1;93m1\x1b[0m..."); io::stdout().flush()?;
    std::thread::sleep(Duration::from_secs(1));
    println!("\rStarting...     \n");
    dl_files().unwrap();

    println!("\n\x1b[1;94mDone downloading files.\x1b[0m\n");
    std::thread::sleep(Duration::from_secs(1));

    create_links(target_path).expect("Failed to create symlinks for pak files");

    println!("\n\x1b[1;94mCreated symlinks for paks.\x1b[0m\n");
    std::thread::sleep(Duration::from_secs(1));

    println!("\n\x1b[1;94mCopying rest of the files.\x1b[0m\n");
    rest_of_files(target_path).unwrap();
    std::thread::sleep(Duration::from_secs(1));

    println!("\n\x1b[1;94mInstalling dependencies...\x1b[0m\n");
    install_deps()?;
    println!("\n\x1b[1;94mDone installing dependencies.\x1b[0m\n");
    std::thread::sleep(Duration::from_secs(1));

    println!("\n\x1b[1;94mCompiling iw1x-server...\x1b[0m\n");
    compile_iw1x(target_path)?;
    std::thread::sleep(Duration::from_secs(1));

    println!("\n\x1b[1;94mCreating start script...\x1b[0m\n");
    create_start_script(target_path)?;
    std::thread::sleep(Duration::from_secs(1));

    println!("\n\x1b[1;94mAll done.\x1b[0m\n");

    Ok(())
}

fn create_dirs(targetdir: &Path) -> io::Result<()>
{
    let datadir = Path::new(&std::env::var("HOME").unwrap())
        .join(".local/share/orbix/main");
    create_dir_all(datadir)?;
    create_dir_all(targetdir.join("main"))?;

    Ok(())
}


fn create_links(dst: &Path) -> io::Result<()>
{
    let srcdir = std::path::Path::new(&std::env::var("HOME").unwrap())
        .join(".local/share/orbix/main");

    let maindir = dst.join("main");

    for entry in read_dir(srcdir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            // cfg file should be copied
            // but we won't do that in this function
            if path.extension().unwrap() != "pk3" {
                continue;
            }
            let file_name = path.file_name().unwrap();
            let target_link = maindir.join(file_name);
            println!("Linking \x1b[1;96m{}\x1b[0m → \x1b[1;95m{}\x1b[0m", path.to_str().unwrap(), target_link.to_str().unwrap());
            if !target_link.exists() {
                std::os::unix::fs::symlink(&path, &target_link)?;
            }
        }
    }

    Ok(())
}

fn rest_of_files(targetdir: &Path) -> io::Result<()>
{
    let svpath = std::path::Path::new(&std::env::var("HOME").unwrap())
        .join(".local/share/orbix/cod-lnxded-1.1d.tar.bz2");
    let cfgpath = svpath.parent().unwrap().join("main/myserver.cfg");

    copy(cfgpath, targetdir.join("main/myserver.cfg"))?;
    copy(svpath, targetdir.join("cod-lnxded-1.1d.tar.bz2"))?;
    extract_tar(&targetdir.join("cod-lnxded-1.1d.tar.bz2"), targetdir)?;
    remove_file(targetdir.join("cod-lnxded-1.1d.tar.bz2"))?;
    Ok(())
}

fn dl_files() -> io::Result<()>
{
    let basefiles = vec![
        ("localized_english_pak0.pk3", "f103bd8e20785ecaed1fed5be0c0fe16"),
        ("localized_english_pak1.pk3", "d2d481075ab68cb579693501a0b5e900"),
        ("pak0.pk3", "03b8bd99d7a5ba7d02a456cbe83fc1a9"),
        ("pak1.pk3", "88d699f4b6a4dd0af5dc42711205c192"),
        ("pak2.pk3", "4079eee442ad0b649e97a5f9258cbc2a"),
        ("pak3.pk3", "61db72b5728fb70b0f2b4d0f1ef4c804"),
        ("pak4.pk3", "c077d4be6cb56e88417f58b516690fb9"),
        ("pak5.pk3", "0cb20baa66ddecc72ccb7f17b3062bb3"),
        ("pak6.pk3", "d8b721be71c0f21fc98901c12cccf3ec"),
    ];

    let path_ = std::path::Path::new(&env::var("HOME").unwrap())
        .join(".local/share/orbix");
    for (file, sum) in &basefiles {
        let outpath = path_.join("main").join(&file);

        while !verify_file(sum, &outpath).unwrap() {
            dl_file(format!("https://de.dvotx.org/dump/cod1/basefiles/{}", &file).as_str(), &outpath).expect("Failed to download basefiles");
        }
    }
    let cfgpath = path_.join("main/myserver.cfg");
    let cfgsum = "f806911639a054ff223f6f78bc9e6e27";
    while !verify_file(cfgsum, &cfgpath).unwrap() {
        dl_file("https://de.dvotx.org/dump/cod1/myserver.cfg", &cfgpath).expect("Failed to download myserver.cfg");
    }

    let svpath = path_.join("cod-lnxded-1.1d.tar.bz2");
    let svsum = "5269ed44b0da8692cd5e61c76b6823d7";
    while !verify_file(svsum, &svpath).unwrap() {
        dl_file("https://de.dvotx.org/dump/cod1/cod-lnxded-1.1d.tar.bz2", &svpath).expect("Failed to download cod-lnxded-1.1d.tar.bz2");
    }
    Ok(())
}

fn verify_file(expected: &str, fpath: &Path) -> io::Result<bool>
{
    print!("[    --    ] Verifying file: {} ", &fpath.to_str().unwrap());
    io::stdout().flush()?;
    if !fpath.is_file() {
        println!("\r{} Verifying file: {}  ", STATUS_FAILED, &fpath.to_str().unwrap());
        return Ok(false);
    }
    let mut file = File::open(fpath)?;
    let mut context = Context::new();
    let mut buffer = [0; 4096];

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 { break; }
        context.consume(&buffer[..bytes_read]);
    }

    let hash = format!("{:x}", context.compute());
    //println!("{}", hash.as_str());
    if hash != expected {
        println!("\r{} Verifying file: {}  ", STATUS_FAILED, &fpath.to_str().unwrap());
        return Ok(false);
    }
    println!("\r{} Verifying file: {}  ", STATUS_OK, &fpath.to_str().unwrap());
    Ok(true)
}

fn dl_file(url: &str, outpath: &Path) -> Result<(), Box<dyn std::error::Error>>
{
    let mut response = get(url)?;
    let total_size = response.content_length().ok_or("Content-Length not found")?;

    let pb = ProgressBar::new(total_size);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})",
        )
        .unwrap()
        .progress_chars("✦➤✧"),
    );

    let mut file = File::create(outpath)?;
    let mut downloaded: u64 = 0;
    let mut buffer = [0; 8192];
    println!("Downloading {}", outpath.file_name().unwrap().to_str().unwrap());

    loop {
        let bytes_read = response.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        file.write_all(&buffer[..bytes_read])?;
        downloaded += bytes_read as u64;
        pb.set_position(downloaded);
    }

    pb.finish_with_message("downloaded");

    Ok(())
}

fn extract_tar(fpath: &Path, outdir: &Path) -> io::Result<()>
{
    let f = File::open(fpath)?;
    let decoder = BzDecoder::new(f);
    let mut archive = Archive::new(decoder);

    archive.unpack(outdir)?;
    Ok(())
}

fn install_deps() -> io::Result<()>
{
    write_scripts().expect("Failed to extract helper scripts");
    let unknown = String::from("unknown");
    let tmp = System::distribution_id_like();
    let tmp = tmp.first().unwrap_or(&unknown);

    let id_like = tmp.as_str();
    let cmd: &str;

    let path_ = std::path::Path::new(&env::var("HOME").unwrap())
        .join(".local/share/orbix");
    let debcmd = path_.join("deps_debian");
    let archcmd = path_.join("deps_debian");

    match id_like {
        "debian" => cmd = debcmd.to_str().unwrap(),
        "arch" => cmd = archcmd.to_str().unwrap(),
        _ => {
            eprintln!("Unsupported distribution: {}", id_like);
            return Err(io::Error::new(io::ErrorKind::Unsupported, "Unsupported OS")); // or handle the error as needed
        }
    }

    Command::new("sudo")
        .arg("bash")
        .arg(cmd)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .expect("Failed to spawn sudo");
    Ok(())
}

fn compile_iw1x(targetdir: &Path) -> io::Result<()>
{
    let path_ = std::path::Path::new(&env::var("HOME").unwrap())
        .join(".local/share/orbix");

    let script = path_.join("compile_iw1x");
    let cmd: &str = script.to_str().unwrap();
    Command::new("bash")
        .arg("-c")
        .arg(cmd)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    let binpath = path_.join("iw1x-server/bin");
    copy(binpath.join("iw1x.so"), targetdir.join("iw1x.so")).expect(&format!("Failed to copy {}/iw1x.so to {}", binpath.to_str().unwrap(), targetdir.to_str().unwrap()));

    Ok(())
}

fn write_scripts() -> io::Result<()>
{
    let path_ = std::path::Path::new(&env::var("HOME").unwrap())
        .join(".local/share/orbix");

    let debscript = path_.join("deps_debian");
    let archscript = path_.join("deps_arch");
    let iw1x_script = path_.join("compile_iw1x");

    if !debscript.exists() {
        write(debscript.to_str().unwrap(), DEPS_DEBIAN)?;
        exe_perm(debscript.to_str().unwrap())?;
    }
    if !archscript.exists() {
        write(archscript.to_str().unwrap(), DEPS_ARCH)?;
        exe_perm(archscript.to_str().unwrap())?;
    }
    if !iw1x_script.exists() {
        write(iw1x_script.to_str().unwrap(), COMPILE_IW1X)?;
        exe_perm(iw1x_script.to_str().unwrap())?;
    }

    Ok(())
}

fn exe_perm(file: &str) -> io::Result<()>
{
    Command::new("bash")
        .arg("-c")
        .arg(format!("chmod +x {}", file))
        .spawn()?;
    Ok(())
}

// AI gen
fn create_start_script(targetdir: &Path) -> io::Result<()> {
    let script_path = targetdir.join("start_server.sh");

    // Fill in the variables
    let preload = targetdir.join("iw1x.so");
    let exe = targetdir.join("cod_lnxded");

    let script = format!(
        "#!/bin/bash\n\
LD_PRELOAD=\"{}\" \"{}\" +set fs_homepath \"{}\" +set fs_basepath \"{}\" \
+set developer_script 1 +exec myserver.cfg\n",
        preload.to_str().unwrap(),
        exe.to_str().unwrap(),
        targetdir.to_str().unwrap(),
        targetdir.to_str().unwrap()
    );

    // Write the file (creates or truncates)
    let mut file = OpenOptions::new()
    .write(true)
    .create(true)
    .truncate(true)
    // Set mode to 0o755 so it's executable
    .mode(0o755)
    .open(&script_path)?;

    file.write_all(script.as_bytes())?;
    file.sync_all()?; // optional flush to disk

    Ok(())
}
