static STATUS_OK: &str      = "[   \x1b[1;92m OK \x1b[0m   ]";
static STATUS_FAILED: &str  = "[ \x1b[1;91m FAILED \x1b[0m ]";
const DEPS_DEBIAN: &[u8] = include_bytes!("scripts/deps_debian");
const DEPS_ARCH: &[u8] = include_bytes!("scripts/deps_arch");
const COMPILE_IW1X: &[u8] = include_bytes!("scripts/compile_iw1x");

use std::{io, env};
use io::{Read, Write};
use std::fs::{OpenOptions, read_to_string, File, write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::process::{Command, Stdio};
use bzip2::read::BzDecoder;
use tar::Archive;
use reqwest::blocking::get;
use indicatif::{ProgressBar, ProgressStyle};
use md5::Context;

use crate::tz_info;

pub(crate) fn verify_file(expected: &str, fpath: &Path) -> io::Result<bool>
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

pub(crate) fn dl_file(url: &str, outpath: &Path) -> Result<(), Box<dyn std::error::Error>>
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

pub(crate) fn extract_tar(fpath: &Path, outdir: &Path) -> io::Result<()>
{
    let f = File::open(fpath)?;
    let decoder = BzDecoder::new(f);
    let mut archive = Archive::new(decoder);

    archive.unpack(outdir)?;
    Ok(())
}

pub(crate) fn exe_perm(file: &str) -> io::Result<()>
{
    Command::new("bash")
        .arg("-c")
        .arg(format!("chmod +x {}", file))
        .spawn()?;
    Ok(())
}

pub(crate) fn install_deps() -> io::Result<()>
{
    write_scripts().expect("Failed to extract helper scripts");
    let cmd: &str;

    let path_ = std::path::Path::new(&env::var("HOME").unwrap())
    .join(".local/share/orbix");
    let debcmd = path_.join("deps_debian");
    let archcmd = path_.join("deps_arch");

    let id = distro_id()?;

    match id.as_str() {
        "debian" => cmd = debcmd.to_str().unwrap(),
        "arch" => cmd = archcmd.to_str().unwrap(),
        _ => {
            eprintln!("Unsupported distribution: {}", id);
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

pub(crate) fn write_scripts() -> io::Result<()>
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

pub(crate) fn distro_info() -> io::Result<Vec<(String, String)>>
{
    let fpath = Path::new("/etc/os-release");
    let data = read_to_string(fpath)?;
    let pairs: Vec<(String, String)> = data.lines() // Split the input string into lines
    .filter_map(|line| {
        let mut parts = line.splitn(2, '='); // Split each line into key and value
        match (parts.next(), parts.next()) {
            (Some(key), Some(value)) => {
                let trimmed_key = key.trim(); // Trim the key
                let trimmed_value = value.trim(); // Trim the value

                // Remove quotes from the value if they exist
                let final_value = if trimmed_value.starts_with('"') && trimmed_value.ends_with('"') {
                    &trimmed_value[1..trimmed_value.len() - 1] // Remove surrounding quotes
                } else {
                    trimmed_value
                };

                Some((trimmed_key.to_string(), final_value.to_string())) // Return the key and processed value
            }
            _ => None, // Ignore lines that don't have a valid key-value pair
        }
    })
    .collect();

    Ok(pairs)
}

pub(crate) fn distro_id() -> io::Result<String>
{
    let mut id = String::new();
    let mut id_like = String::new();
    let pairs = distro_info()?;
    for (key, value) in pairs {
        if key == "ID" {
            id = value.clone();
        }
        if key == "ID_LIKE" {
            id_like = value;
        }
    }
    if !id_like.is_empty() {
        return Ok(id_like);
    }
    if !id.is_empty() {
        return Ok(id);
    }

    Ok(String::from("unknown"))
}

pub(crate) fn create_cfg(targetdir: &Path) -> io::Result<()>
{
    let mut distro: String = String::new();
    let pairs = distro_info()?;
    for (key, value) in pairs {
        if key == "PRETTY_NAME" {
            distro = value;
        }
    }
    let tz_name = iana_time_zone::get_timezone().unwrap();
    let country = tz_info::get_country(&tz_name.split("/").nth(1).unwrap().trim())?;

    let cfg = format!(
        r#"
// Add server to xFire
set gamename "Call of Duty"

// Developer settings
set developer "0"
set developer_script "0" // printLn() print to console

// Server information (visible public/getstatus)
sets ^1Owner "<OWNER NAME>"
sets ^1Distro "{distro}"
sets ^1Location "^2{country}"

// Server options
set sv_maxclients "16"
set sv_hostname "<SERVER NAME>"
set scr_motd "Free Palestine!"
set sv_pure "0"
set g_gametype "sd"
set sv_maprotation "gametype sd map mp_harbor"

set rconpassword "<RCON PW HERE>"
set g_password "<JOIN PW HERE>"
set sv_privatepassword ""

set sv_privateclients "0"
set sv_allowdownload "0"
set sv_cheats "0"

set g_log "" // logPrint() logfile (default games_mp.log)
set g_logsync "0"
set logfile "0" // "1" output console to console_mp_server.log file

set sv_fps "20"
set sv_allowanonymous "0"
set sv_floodprotect "1"
set g_inactivity "0"

// Network options
set sv_maxrate "0"
set sv_maxping "0"
set sv_minping "0"

// Additional masterservers (up to sv_master5, sv_master1 default to Activision)
set sv_master2 "master.cod.pm"

// Game options (stock gametypes)
set g_allowvote "0"
set scr_allow_vote "0"
set scr_drawfriend "0"
set scr_forcerespawn "0"
set scr_friendlyfire "0"

// Deathmatch
set scr_dm_scorelimit "50"
set scr_dm_timelimit "30"

// Team Deathmatch
set scr_tdm_scorelimit "100"
set scr_tdm_timelimit "30"

// Behind Enemy Lines
set scr_bel_scorelimit "50"
set scr_bel_timelimit "30"
set scr_bel_alivepointtime "10"

// Retrieval
set scr_re_scorelimit "10"
set scr_re_timelimit "0"
set scr_re_graceperiod "15"
set scr_re_roundlength "2.50"
set scr_re_roundlimit "0"
set scr_re_showcarrier "0"

// Search and Destroy
set scr_sd_scorelimit "10"
set scr_sd_timelimit "0"
set scr_sd_graceperiod "20"
set scr_sd_roundlength "2.50"
set scr_sd_roundlimit "0"

// Weapons
set scr_allow_m1carbine "1"
set scr_allow_m1garand "1"
set scr_allow_enfield "1"
set scr_allow_bar "1"
set scr_allow_bren "1"
set scr_allow_mp40 "1"
set scr_allow_mp44 "1"
set scr_allow_sten "1"
set scr_allow_ppsh "1"
set scr_allow_fg42 "1"
set scr_allow_thompson "1"
set scr_allow_panzerfaust "1"
set scr_allow_springfield "1"
set scr_allow_kar98ksniper "1"
set scr_allow_nagantsniper "1"
set scr_allow_kar98k "1"
set scr_allow_nagant "1"

//set scr_allow_mg42 "1" // CoDaM setting

// IW1X

// Jump
set airjump_heightScale "2"

jump_bounceEnable "0" // 1 to enable bouncing

jump_height "39"


set fs_callbacks "maps/mp/gametypes/_callbacksetup"
set fs_callbacks_additional "" // set to "callback" for using miscmod chat commands
set fs_svrPaks "" // files that should NOT be downloaded, separate by semi-colon, omit the .pk3 extension

// Game
set g_deadChat "1" // allow dead players to chat
set g_debugCallbacks "0" // use for debugging callbacks
set g_playerEject "0" // eject stuck players, prevents players from standing on each other
set g_resetSlide "0" // stop sliding after fall damage

// Server

set sv_botHook "0" // Bot gsc functions

set sv_connectMessage "" // speaks for itself
set sv_connectMessageChallenges "1" // the number of challenges to show connect messages

set sv_cracked "0" // 1 to make the server cracked

set sv_debugRate "0" // use for debugging rate

set sv_heartbeatDelay "180" // custom heartbeat delay in seconds

set sv_statusShowDeath "1"
set sv_statusShowTeamScore "1"

set sv_spectatorNoclip "0"

// Download
set sv_downloadForce "0"
set sv_downloadNotifications "1"
set sv_fastDownload "1"

// Sprint
set player_sprint "0" // 1 to enable sprinting
set player_sprintMinTime "1" // minimum sprint time
set player_sprintTime "4" // Sprint time
set player_sprintSpeedScale "1.4" // sprint speed scale (g_speed * scale)

// VM
//set bg_fallDamageMaxHeight ""
//set bg_fallDamageMinHeight ""

// Execute CoDaM Configuration

//exec CoDaM.cfg
//exec CoDaM_HamGoodies.cfg
//exec CoDaM_MiscMod.cfg

"#
    );

    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(targetdir)?;

    file.write_all(cfg.as_bytes())?;
    file.sync_all()?;
    Ok(())
}

// AI gen
pub(crate) fn create_start_script(targetdir: &Path) -> io::Result<()> {
    let script_path = targetdir.join("start.sh");

    // Fill in the variables
    let preload = targetdir.join("iw1x.so");
    let exe = targetdir.join("cod_lnxded");

    let script = format!(
        "#!/bin/bash\n\
LD_PRELOAD=\"{}\" \"{}\" +set fs_homepath \"{}\" +set fs_basepath \"{}\" \
+set developer_script 1 +exec myserver.cfg +map_rotate\n",
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
