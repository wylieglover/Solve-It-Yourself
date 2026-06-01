use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::{
    io::{BufRead, BufReader, Seek, SeekFrom},
    thread::sleep,
    time::{Duration, SystemTime},
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use ctrlc;
use std::process::Command;

const BLOCK_SITES_PATH: &str = "block_sites.txt";

const HOST_FILE_PATH: &str = "/mnt/c/Windows/System32/drivers/etc/hosts";

fn open_hosts_file() -> Result<File, std::io::Error> {
    let mut path = PathBuf::new();
    path.push(HOST_FILE_PATH);

    Ok(File::options().read(true).append(true).open(path)?)
}

fn get_reader_for_block_sites_file(
) -> Result<BufReader<File>, std::io::Error> {
    let input_file = File::open(BLOCK_SITES_PATH)?;
    let block_sites_reader = BufReader::new(input_file);
    Ok(block_sites_reader)
}

fn get_reader_for_hosts_file(host_file: &File) -> Result<BufReader<&File>, std::io::Error> {
    let host_reader = BufReader::new(host_file);
    Ok(host_reader)
}

fn get_writer_for_hosts_file(host_file: &File) -> Result<BufWriter<&File>, std::io::Error> {
    let host_writer = BufWriter::new(host_file);
    Ok(host_writer)
}

fn block_ai_sites(
    block_sites_reader: &mut BufReader<File>,
    host_file_writer: &mut BufWriter<&File>,
) -> Result<(), std::io::Error> {
    let mut line = String::new();

    while block_sites_reader.read_line(&mut line)? > 0 {
        host_file_writer.write_all(line.as_bytes())?;
        line.clear();
    }

    host_file_writer.flush()?;

    Ok(())
}

fn get_host_file_contents(
    mut host_file_reader: BufReader<&File>,
) -> Result<Vec<String>, std::io::Error> {
    host_file_reader.seek(SeekFrom::Start(0))?;
    
    let mut host_file_contents: Vec<String> = Vec::new();
    for line in host_file_reader.lines() {
        let line = line?;
        host_file_contents.push(line);
    }

    Ok(host_file_contents)
}

fn unblock_ai_sites(
    block_sites_reader: &mut BufReader<File>,
    host_file_contents: Vec<String>,
) -> Result<(), std::io::Error> {
    block_sites_reader.seek(SeekFrom::Start(0))?;
    
    let mut block_sites_content: Vec<String> = Vec::new();

    for line in block_sites_reader.lines() {
        let line = line?;
        block_sites_content.push(line);
    }

    let file_contents = host_file_contents
        .iter()
        .map(|s| s.to_string())
        .filter(|line| !block_sites_content.contains(line))
        .collect::<Vec<String>>()
        .join("\n");

    std::fs::write(HOST_FILE_PATH, file_contents)?;
    Ok(())
}


fn cmd_status(mut cmd: Command) {
    let status = cmd.status()
        .expect("Failed to execute command");

    if status.success() {
        println!("Command finished successfully.");
    } else {
        eprintln!("Command failed with exit code: {:?}", status.code());
    }
}
fn execute_ipconfig_flush() {
    let mut cmd = Command::new("ipconfig.exe");
    cmd.arg("/flushdns");

    cmd_status(cmd);
}

fn execute_taskkill_on_chrome() {
    let mut cmd = Command::new("taskkill.exe");
    cmd.arg("/F"); // Forcefully kills the processes
    cmd.arg("/IM"); // Targets all instances of the Chrome executable
    cmd.arg("chrome.exe"); // Target (Chrome executable)
    cmd.arg("/T"); // Kills the process along with any associated child processes

    cmd_status(cmd);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        println!("\nReceived Ctrl+C! Signaling main loop to stop...");
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    let current_time = SystemTime::now();
    let end_time = Duration::from_mins(20);

    let host_file = open_hosts_file()?;
    let mut block_sites_reader = get_reader_for_block_sites_file()?;
    let mut host_file_writer = get_writer_for_hosts_file(&host_file)?;

    block_ai_sites(&mut block_sites_reader, &mut host_file_writer)?;
    execute_ipconfig_flush();
    execute_taskkill_on_chrome();

    while running.load(Ordering::SeqCst) && current_time.elapsed()? < end_time {
        sleep(Duration::from_secs(1));
    }

    print!("Cleaning up host file...");

    let host_file_reader = get_reader_for_hosts_file(&host_file)?;
    let host_file_contents = get_host_file_contents(host_file_reader)?;

    unblock_ai_sites(&mut block_sites_reader, host_file_contents)?;
    
    Ok(())
}
