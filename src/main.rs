/*
 * Copyright (c) 2021 Sylvain "Skarsnik" Colinet
 *
 * This file is part of the usb2snes-cli project.
 * (see https://github.com/usb2snes/usb2snes-cli).
 *
 * usb2snes-cli is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * usb2snes-cli is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with QUsb2Snes.  If not, see <https://www.gnu.org/licenses/>.
 */


use structopt::StructOpt;
use std::thread::sleep;
use std::io::prelude::*;
use std::fs::File;
use std::fs;
use std::path::Path;
use std::time::Duration;
use scan_fmt::scan_fmt;
use crate::usb2snes::usb2snes::SyncClient;

mod usb2snes;


#[derive(StructOpt, Debug)]
#[structopt(name = "usb2snes-cli", about = "usb2snes-cli --boot /games/Super Metroid.smc")]
struct Opt {
    //#[structopt(long, help = "Operate in silent mode")]
    //quiet:  Option<bool>,

    #[structopt(long, name = "list", help = "List the device available")]
    list_device: bool,
    #[structopt(long, name = "list-loop", help = "List the device every second")]
    list_device_loop: bool,

    #[structopt(long, help = "Use the specified device")]
    device: Option<String>,

    #[structopt(long = "get-address", help = "Read a usb2snes address, syntax address_in_hex:size")]
    get_address : Option<String>,

    #[structopt(long, help = "Reset the game running on the device")]
    reset: bool,
    #[structopt(long, help = "Bring back the sd2snes/fxpak pro to the menu")]
    menu: bool,
    #[structopt(long, name = "File to boot", help = "Boot the specified file")]
    boot: Option<String>,
    
    #[structopt(long = "ls", name = "List the specified directory", help = "List the specified directory, path separator is /")]
    ls_path: Option<String>,

    #[structopt(long = "upload", name = "File to upload", help = "Upload a file to the device, use --path to specify the path on the device, like --upload SM.smc --path=/games/Super Metroid.smc")]
    file_to_upload: Option<String>,

    #[structopt(long = "path", name = "The path on the device")]
    path: Option<String>,

    #[structopt(long = "download", name = "File to download")]
    file_to_download: Option<String>,

    #[structopt(long = "rm", name = "Path on the device of a file to remove")]
    path_to_remove: Option<String>,

    #[structopt(long = "devel", name = "Show all the transaction with the usb2snes server")]
    devel: bool,

    #[structopt(subcommand)]
    command: Option<Command>
}

#[derive(StructOpt, Debug)]
enum Command {
    #[structopt(name = "upload-latest-sfc", about = "Find the most recent .sfc file in local-source dir, and upload it to target-dir on the device")]
    UploadLatestSfc {
        #[structopt(name = "local-source-dir", help = "Directory on this computer to get the latest .sfc out of, e.g. your downloads folder")]
        local_source_dir: String,

        #[structopt(name = "target-dir", help = "Directory on the device to put the .sfc into")]
        target_dir: String,

        #[structopt(long = "wipe-target-dir", help = "Delete .sfc files in target-dir before copying a new one there")]
        wipe_target_dir: bool
    }
}

fn main() {
    let opt = Opt::from_args();

    let mut usb2snes;
    if opt.devel {
        usb2snes = usb2snes::usb2snes::SyncClient::connect_with_devel();
    } else {
        usb2snes = usb2snes::usb2snes::SyncClient::connect();
    }
    println!("Connected to the Usb2snes server");
    usb2snes.set_name(String::from("usb2snes-cli"));
    println!("Server version is : {:?}", usb2snes.app_version());

    let mut devices = usb2snes.list_device();

    if opt.list_device_loop {
        loop {
            println!("Devices : {:?}", devices);
            sleep(Duration::new(0, 5000000));
            devices = usb2snes.list_device();
        }
    }
    if devices.is_empty() {
        println!("No device found");
        std::process::exit(1);
    } else {
        if opt.list_device {
            println!("Listing devices :");
            for dev in devices {
                let mut infoclient = usb2snes::usb2snes::SyncClient::connect();
                infoclient.attach(&dev);
                let info = infoclient.info();
                println!("For device : {:?}, type : {:?}, version :{:?}, game : {:?} - Flags : {:?}", dev, info.dev_type, info.version, info.game, info.flags);
            }
            std::process::exit(0);
        }
        let device = opt.device.unwrap_or_else(||devices[0].clone());
        if !devices.contains(&device) {
            println!("Can't find the specified device <{:?}>", &device);
            std::process::exit(1);
        }
        usb2snes.attach(&device);
        let info = usb2snes.info();
        if info.flags.contains(&String::from("NO_FILE_CMD")) && (opt.file_to_download != None || opt.file_to_upload != None || opt.ls_path != None) {
            println!("The device does not support file commands");
            std::process::exit(1);
        }
        if info.flags.contains(&String::from("NO_CONTROL_CMD")) && (opt.menu || opt.reset || opt.boot != None) {
            println!("The device does not support control command (menu/reset/boot)");
            std::process::exit(1);
        }
        if opt.get_address != None {
            let toget = opt.get_address.unwrap();
            if let Ok((address, size)) = scan_fmt!(&toget, "{x}:{d}", [hex u32], usize) {
                let data = usb2snes.get_address(address, size);
                let mut i = 0;
                while i < data.len() {
                    if i % 16 == 0 {
                        println!();
                        print!("{:02X} : ", i);
                    }
                    print!("{:02X} ", data[i]);
                    i += 1;
                }
            }
        }
        if opt.menu {
            usb2snes.menu();
        }
        if opt.boot != None {
            usb2snes.boot(&opt.boot.unwrap());
        }
        if opt.reset {
            usb2snes.reset();
        }
        if opt.ls_path != None {
            let path = opt.ls_path.unwrap().to_string();
            let dir_infos = usb2snes.ls(&path);
            println!("Listing {:?} : ", path);
            for info in dir_infos {
                println!("{:?}", format!("{}{}", info.name, if info.file_type == usb2snes::usb2snes::USB2SnesFileType::Dir {"/"} else {""}));
            }
        }
        if opt.file_to_upload != None {
            if opt.path == None {
                println!("You need to provide a --path to upload a file");
                std::process::exit(1);
            }
            let local_path = opt.file_to_upload.unwrap();
            let snes_path = opt.path.unwrap();
            upload_file(local_path, snes_path, &mut usb2snes);
        }
        if opt.file_to_download != None {
            let path:String = opt.file_to_download.unwrap();
            let local_path = path.split('/').last().unwrap();
            println!("Downloading : {:?} , local file {:?}", path, local_path);
            let data = usb2snes.get_file(&path);
            let f = File::create(local_path);
            let mut f = match f {
                Ok(file) => file,
                Err(err) => panic!("Problem opening the file {:?} : {:?}", path, err),
            };
            f.write_all(&data).expect("Can't write the data to the file");
        }
        if opt.path_to_remove != None {
            let path:String = opt.path_to_remove.unwrap();
            println!("Removing : {:?}", path);
            usb2snes.remove_path(&path);
        }
        match opt.command {
            Some(Command::UploadLatestSfc {local_source_dir, target_dir, wipe_target_dir  }) => {
                    do_upload_latest_sfc(&mut usb2snes, local_source_dir, target_dir, wipe_target_dir).unwrap();
            },
            None => {}
        }

    }
}

fn upload_file(local_path:String, snes_path:String, usb2snes: &mut SyncClient) {
    print!("Sending file {} to snes at {}", local_path, snes_path);
    let data = fs::read(local_path).expect("Error opening the file or reading the content");
    usb2snes.send_file(&snes_path, data);
}

fn do_upload_latest_sfc(usb2snes: &mut SyncClient, local_source_dir:String, target_dir:String, wipe_target_dir:bool)
                        -> Result<(), Box<dyn std::error::Error>> {
    println!("Uploading latest sfc from {:?} to {:?}. with wipe-target-dir={:?}",
             local_source_dir,
             target_dir,
             wipe_target_dir);
    let local_dir_path = Path::new(&local_source_dir);

    let mut newest_seconds_since_mod = u64::MAX;
    let mut newest_file_name:Option<String> = None;
    for entry in fs::read_dir(local_dir_path)
        .expect(&format!("Can't read the given local dir {:?}", local_source_dir)) {
        let entry = entry?;

        let file_path = entry.path();
        let f_name = entry.file_name().to_string_lossy().into_owned();
        if !f_name.ends_with(".sfc") {
            continue
        }

        let metadata = fs::metadata(&file_path)?;
        let seconds_since_mod = metadata.modified()?.elapsed()?.as_secs();
        if seconds_since_mod < newest_seconds_since_mod {
            newest_seconds_since_mod = seconds_since_mod;
            newest_file_name = Some(f_name);
        }
    }
    let file_name_to_send = newest_file_name.expect("No sfc found in local dir");
    println!("Newest sfc file found: {:?}", file_name_to_send);

    if wipe_target_dir {
        let roms = usb2snes.ls(&target_dir);
        for rom in roms {
            if rom.name.ends_with(".sfc") {
                let snes_path = format!("{}/{}", target_dir, rom.name);
                println!("Deleting snes file {}", snes_path);
                usb2snes.remove_path(&snes_path);
            }
        }

    }

    let local_path:String = format!("{}/{}", local_source_dir, file_name_to_send);
    let snes_path:String = format!("{}/{}", target_dir, file_name_to_send);
    upload_file(local_path, snes_path, usb2snes);

    Ok(())
}
