use core::time::Duration;
use io::ErrorKind;
use postcard::{from_bytes, to_stdvec};
use protocol::{Request, Response};
use std::io::{self, Write};

// // use clap::{App, AppSettings, Arg};

fn main() {
    // let matches = App::new("Serialport Example - Receive Data")
    //     .about("Reads data from a serial port and echoes it to stdout")
    //     .setting(AppSettings::DisableVersion)
    //     .arg(
    //         Arg::with_name("port")
    //             .help("The device path to a serial port")
    //             .use_delimiter(false)
    //             .required(true),
    //     )
    //     .arg(
    //         Arg::with_name("baud")
    //             .help("The baud rate to connect at")
    //             .use_delimiter(false)
    //             .required(true)
    //             .validator(valid_baud),
    //     )
    //     .get_matches();
    // let port_name = matches.value_of("port").unwrap();
    // let baud_rate = matches.value_of("baud").unwrap().parse::<u32>().unwrap();
    let port_name = "/dev/ttyACM0";
    let baud_rate = 1_000_000;

    let port = serialport::new(port_name, baud_rate)
        .timeout(Duration::from_millis(2000))
        .open();

    match port {
        Ok(mut port) => {
            let mut serial_buf: Vec<u8> = vec![0; 2048];
            println!("Receiving data on {} at {} baud:", &port_name, &baud_rate);
            let mut i = 0u32;
            let mut is_waiting = false;
            loop {
                if !is_waiting {
                    let res = i % 5;
                    let request = if res == 0 {
                        Request::Serial
                    } else if res == 1 {
                        Request::Info
                    // Request::Ping
                    } else if res == 2 {
                        Request::Address(i - res)
                    } else if res == 3 {
                        Request::AddressList(i - res)
                    } else {
                        Request::Sig(&[0x41, 0x42, 0x43, 0x44])
                    };
                    // let request = Request::AddressList(i * 5);
                    let data = to_stdvec(&request).unwrap();
                    match port.write(&data) {
                        Ok(count) => println!("Sent ({}): {:?}", count, &request),
                        Err(e) => eprintln!("err'd with {:?}", e),
                    };
                }
                // Wait for the response
                match port.read(serial_buf.as_mut_slice()) {
                    Ok(t) => {
                        is_waiting = false;
                        // println!("Parsed ({}): {}", t, hex::encode(&serial_buf[..]));
                        // Final byte is the version, assert it's the same
                        let _ver = &serial_buf[t - 1..t][0];
                        // assert_eq!(*ver, protocol::version());
                        if let Ok(response) = from_bytes::<Response>(&serial_buf[..t - 1]) {
                            println!("Rcvd({}): {}", t, response);
                            i += 1;
                        } else {
                            println!("Failed to parse ({}): {}", t, hex::encode(&serial_buf[..]));
                        }
                        // println!("Rcvd({}): {}", t, hex::encode(&serial_buf[..]));
                    }
                    Err(e) if e.kind() == ErrorKind::TimedOut => {
                        // eprint!("timed out, waiting...");
                        // std::thread::sleep(Duration::from_secs(5))
                        is_waiting = true;
                    }
                    Err(e) => {
                        eprintln!("errored while receiving with: {:?}", e);
                        break;
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to open \"{}\". Error: {}", port_name, e);
            ::std::process::exit(1);
        }
    }
}
