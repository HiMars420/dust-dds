use std::net::UdpSocket;
use std::net::Ipv4Addr;
use std::str::FromStr;
use std::net::AddrParseError;
use std::net::SocketAddr;
use std::net::SocketAddrV4;
use std::time::Duration;

use crate::parser::{RtpsMessage, parse_rtps_message};

struct Endpoint {
    socket: UdpSocket,
}

impl Endpoint {
    pub fn new(/*multicast_locator: Udpv4Locator*/) -> Endpoint {

        let socket = UdpSocket::bind(SocketAddr::from(([0, 0, 0, 0], 7400))).expect("couldn't bind to address");
        socket.set_multicast_loop_v4(true).expect("Error setting multicast loop");
        let multicast_addr = Ipv4Addr::from_str("239.255.0.1").unwrap();
        let multicast_interface = Ipv4Addr::from_str("192.168.2.4").expect("Error resolving multicast interface address");
        socket.join_multicast_v4(&multicast_addr, &multicast_interface).expect("Error joining multicast group");
        socket.set_read_timeout(Some(Duration::new(1,0))).expect("Error setting timeout");

        Endpoint {
            socket,
        }
    }

    pub fn read_data(&self) -> () {
        let mut buf = [0;512];

        let received = self.socket.recv_from(&mut buf);
        if received.is_ok() {
            println!("Received {:?}", &buf[0 .. 50]);
            let received_message = parse_rtps_message(&buf).unwrap();
            print!("Received message with protocol {:?}", received_message.get_protocol_version());
        } else {
            println!("Didn't receive data within timeout");
        }
    }

}


// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn read_data_from_endpoint() {
//         let udp_discovery_endpoint = Endpoint::new();

//         for _i in 1..=120 {
//             println!("Reading data");
//             udp_discovery_endpoint.read_data();
//         }
//     }
// }

