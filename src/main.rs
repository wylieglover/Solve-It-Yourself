use ctrlc;
use std::{
    collections::HashMap,
    fs,
    net::ToSocketAddrs,
    process::Command,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::sleep,
    time::{Duration, Instant},
};

const BLOCK_SITES_PATH: &str = "block_sites.txt";

fn load_block_sites() -> Result<Vec<String>, std::io::Error> {
    let input_file = fs::read_to_string(BLOCK_SITES_PATH)?;

    Ok(input_file.lines().map(String::from).collect())
}

fn execute_command(cmd: &mut Command) {
    let status = cmd.status().expect("Failed to execute command");

    if status.success() {
        println!("Command finished successfully.");
    } else {
        eprintln!("Command failed with exit code: {:?}", status.code());
    }
}

fn add_firewall_rules(hostname_to_ips: &HashMap<String, Vec<String>>) {
    for (name, ips) in hostname_to_ips {
        for ip in ips {
            let mut cmd = Command::new("netsh.exe");
            cmd.args([
                "advfirewall",
                "firewall",
                "add",
                "rule",
                &format!("name={}", name),
                "dir=out",
                "action=block",
                &format!("remoteip={}", ip),
            ]);
            execute_command(&mut cmd);
        }
    }
}

fn delete_firewall_rules(hostname_to_ips: HashMap<String, Vec<String>>) {
    for (name, _) in hostname_to_ips.iter() {
        let mut cmd = Command::new("netsh.exe");
        cmd.args([
            "advfirewall",
            "firewall",
            "delete",
            "rule",
            &format!("name={name}"),
        ]);
        execute_command(&mut cmd);
    }
}

fn resolve_hostnames(host_names: Vec<String>) -> HashMap<String, Vec<String>> {
    let mut hostname_to_ips: HashMap<String, Vec<String>> = HashMap::new();

    for host in host_names {
        let host_with_port = format!("{}:0", host);
        match host_with_port.to_socket_addrs() {
            Ok(addrs) => {
                let ips: Vec<String> = addrs.map(|addr| addr.ip().to_string()).collect();

                hostname_to_ips.insert(host, ips);
            }
            Err(e) => eprintln!("Resolution failed: {}", e),
        }
    }

    hostname_to_ips
}

fn setup_ctrl_c_handler() -> Arc<AtomicBool> {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        println!("\nReceived Ctrl+C! Signaling main loop to stop...");
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    running
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let running = setup_ctrl_c_handler();

    let start_time = Instant::now();
    let end_time = Duration::from_mins(20);

    let block_sites = load_block_sites()?;
    let hostname_to_ips = resolve_hostnames(block_sites);

    println!("Blocking sites on firewall...");
    add_firewall_rules(&hostname_to_ips);

    while running.load(Ordering::SeqCst) && start_time.elapsed() < end_time {
        sleep(Duration::from_secs(1));
    }

    println!("Timer ended or Ctrl+C caught. Removing firewall rules...");
    delete_firewall_rules(hostname_to_ips);

    Ok(())
}
