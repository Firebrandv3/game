// Standard
use std::{
    collections::{vec_deque::VecDeque, HashMap},
    net::{SocketAddr, TcpStream, ToSocketAddrs, UdpSocket},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
};

// Library
use bincode;
use get_if_addrs::get_if_addrs;
use parking_lot::{Mutex, MutexGuard, RwLock};

// Parent
use super::{
    message::Message,
    packet::{Frame, FrameError, IncommingPacket, OutgoingPacket},
    protocol::Protocol,
    tcp::Tcp,
    udp::Udp,
    udpmgr::UdpMgr,
    Error,
};

pub trait Callback<RM: Message> {
    fn recv(&self, Result<RM, Error>);
}

#[derive(Clone, Debug, Serialize, Deserialize)]
enum ConnectionMessage {
    OpenedUdp { host: SocketAddr },
    Shutdown,
    Ping,
}

impl Message for ConnectionMessage {
    fn from_bytes(data: &[u8]) -> Result<ConnectionMessage, Error> {
        bincode::deserialize(data).map_err(|_e| Error::CannotDeserialize)
    }

    fn to_bytes(&self) -> Result<Vec<u8>, Error> { bincode::serialize(&self).map_err(|_e| Error::CannotSerialize) }
}

pub struct Connection<RM: Message> {
    // sorted by prio and then cronically
    tcp: Tcp,
    udpmgr: Arc<UdpMgr>,
    udp: Mutex<Option<Udp>>,
    callback: Mutex<Box<Fn(Result<RM, Error>) + Send>>,
    callbackobj: Mutex<Option<Arc<Callback<RM> + Send + Sync>>>,
    packet_in: Mutex<HashMap<u64, IncommingPacket>>,
    packet_out: Mutex<Vec<VecDeque<OutgoingPacket>>>,
    packet_out_count: RwLock<u64>,
    running: AtomicBool,
    send_thread: Mutex<Option<JoinHandle<()>>>,
    recv_thread: Mutex<Option<JoinHandle<()>>>,
    send_thread_udp: Mutex<Option<JoinHandle<()>>>,
    recv_thread_udp: Mutex<Option<JoinHandle<()>>>,
    next_id: Mutex<u64>,
}

impl<'a, RM: Message + 'static> Connection<RM> {
    pub fn new<A: ToSocketAddrs>(
        remote: &A,
        callback: Box<Fn(Result<RM, Error>) + Send>,
        cb: Option<Arc<Callback<RM> + Send + Sync>>,
        udpmgr: Arc<UdpMgr>,
    ) -> Result<Arc<Connection<RM>>, Error> {
        Connection::new_internal(Tcp::new(&remote)?, callback, cb, udpmgr)
    }

    pub fn new_stream(
        stream: TcpStream,
        callback: Box<Fn(Result<RM, Error>) + Send>,
        cb: Option<Arc<Callback<RM> + Send + Sync>>,
        udpmgr: Arc<UdpMgr>,
    ) -> Result<Arc<Connection<RM>>, Error> {
        Connection::new_internal(Tcp::new_stream(stream)?, callback, cb, udpmgr)
    }

    fn new_internal(
        tcp: Tcp,
        callback: Box<Fn(Result<RM, Error>) + Send>,
        cb: Option<Arc<Callback<RM> + Send + Sync>>,
        udpmgr: Arc<UdpMgr>,
    ) -> Result<Arc<Connection<RM>>, Error> {
        let packet_in = HashMap::new();
        let mut packet_out = Vec::new();
        for _i in 0..255 {
            packet_out.push(VecDeque::new());
        }

        let m = Connection {
            tcp,
            udpmgr,
            udp: Mutex::new(None),
            callback: Mutex::new(callback),
            callbackobj: Mutex::new(cb),
            packet_in: Mutex::new(packet_in),
            packet_out_count: RwLock::new(0),
            packet_out: Mutex::new(packet_out),
            running: AtomicBool::new(true),
            send_thread: Mutex::new(None),
            recv_thread: Mutex::new(None),
            send_thread_udp: Mutex::new(None),
            recv_thread_udp: Mutex::new(None),
            next_id: Mutex::new(1),
        };

        Ok(Arc::new(m))
    }

    pub fn open_udp<'b>(manager: &'b Arc<Connection<RM>>, listen: SocketAddr, sender: SocketAddr) {
        if let Some(..) = *manager.udp.lock() {
            panic!("not implemented");
        }
        *manager.udp.lock() = Some(Udp::new(listen, sender).unwrap());
        manager.send(ConnectionMessage::OpenedUdp { host: listen });

        let m = manager.clone();
        let mut rt = manager.recv_thread_udp.lock();
        *rt = Some(thread::spawn(move || {
            m.recv_worker_udp();
        }));

        let m = manager.clone();
        let mut st = manager.send_thread_udp.lock();
        *st = Some(thread::spawn(move || {
            m.send_worker_udp();
        }));
    }

    pub fn start<'b>(manager: &'b Arc<Connection<RM>>) {
        let m = manager.clone();
        let mut rt = manager.recv_thread.lock();
        *rt = Some(thread::spawn(move || {
            m.recv_worker();
        }));

        let m = manager.clone();
        let mut st = manager.send_thread.lock();
        *st = Some(thread::spawn(move || {
            m.send_worker();
        }));
    }

    pub fn stop<'b>(manager: &'b Arc<Connection<RM>>) {
        let m = manager.clone();
        m.running.store(false, Ordering::Relaxed);
        // non blocking stop for now
    }

    pub fn send<M: Message>(&self, message: M) {
        let mut id = self.next_id.lock();
        self.packet_out.lock()[16].push_back(OutgoingPacket::new(message, *id));
        *id += 1;
        let mut p = self.packet_out_count.write();
        *p += 1;
        let mut rt = self.send_thread.lock();
        if let Some(cb) = rt.as_mut() {
            //trigger sending
            cb.thread().unpark();
        }
    }

    fn trigger_callback(&self, msg: Result<RM, Error>) {
        //trigger callback
        let f = self.callback.lock();
        let mut co = self.callbackobj.lock();
        match co.as_mut() {
            Some(cb) => {
                cb.recv(msg);
            },
            None => {
                f(msg);
            },
        }
    }

    fn send_worker(&self) {
        loop {
            if !self.running.load(Ordering::Relaxed) {
                break;
            }
            if *self.packet_out_count.read() == 0 {
                thread::park();
                continue;
            }
            // find next package
            let mut packets = self.packet_out.lock();
            for i in 0..255 {
                if packets[i].len() != 0 {
                    // build part
                    const SPLIT_SIZE: u64 = 2000;
                    match packets[i][0].generate_frame(SPLIT_SIZE) {
                        Ok(frame) => {
                            // send it
                            self.tcp.send(frame).unwrap();
                        },
                        Err(FrameError::SendDone) => {
                            packets[i].pop_front();
                            let mut p = self.packet_out_count.write();
                            *p -= 1;
                        },
                    }

                    break;
                }
            }
        }
    }

    fn recv_worker(&self) {
        loop {
            if !self.running.load(Ordering::Relaxed) {
                break;
            }
            let frame = self.tcp.recv();
            match frame {
                Ok(frame) => {
                    match frame {
                        Frame::Header { id, .. } => {
                            let msg = IncommingPacket::new(frame);
                            let mut packets = self.packet_in.lock();
                            packets.insert(id, msg);
                        },
                        Frame::Data { id, .. } => {
                            let mut packets = self.packet_in.lock();
                            let packet = packets.get_mut(&id);
                            if packet.unwrap().load_data_frame(frame) {
                                //convert
                                let packet = packets.get_mut(&id);
                                let data = packet.unwrap().data();
                                debug!("received packet: {:?}", &data);
                                let msg = RM::from_bytes(data);
                                self.trigger_callback(Ok(msg.unwrap()));
                            }
                        },
                    }
                },
                Err(e) => {
                    error!("Net Error {:?}", &e);
                    self.trigger_callback(Err(e));
                },
            }
        }
    }

    fn send_worker_udp(&self) {
        loop {
            if !self.running.load(Ordering::Relaxed) {
                break;
            }
            if *self.packet_out_count.read() == 0 {
                thread::park();
                continue;
            }
            // find next package
            let mut packets = self.packet_out.lock();
            for i in 0..255 {
                if packets[i].len() != 0 {
                    // build part
                    const SPLIT_SIZE: u64 = 2000;
                    match packets[i][0].generate_frame(SPLIT_SIZE) {
                        Ok(frame) => {
                            // send it
                            let mut udp = self.udp.lock();
                            udp.as_mut().unwrap().send(frame).unwrap();
                        },
                        Err(FrameError::SendDone) => {
                            packets[i].pop_front();
                            let mut p = self.packet_out_count.write();
                            *p -= 1;
                        },
                    }

                    break;
                }
            }
        }
    }

    fn recv_worker_udp(&self) {
        loop {
            if !self.running.load(Ordering::Relaxed) {
                break;
            }
            let mut udp = self.udp.lock();
            let frame = udp.as_mut().unwrap().recv();
            match frame {
                Ok(frame) => {
                    match frame {
                        Frame::Header { id, .. } => {
                            let msg = IncommingPacket::new(frame);
                            let mut packets = self.packet_in.lock();
                            packets.insert(id, msg);
                        },
                        Frame::Data { id, .. } => {
                            let mut packets = self.packet_in.lock();
                            let packet = packets.get_mut(&id);
                            if packet.unwrap().load_data_frame(frame) {
                                //convert
                                let packet = packets.get_mut(&id);
                                let data = packet.unwrap().data();
                                debug!("received packet: {:?}", &data);
                                let msg = RM::from_bytes(data);
                                self.trigger_callback(Ok(msg.unwrap()));
                            }
                        },
                    }
                },
                Err(e) => {
                    error!("Net Error {:?}", &e);
                    self.trigger_callback(Err(e));
                },
            }
        }
    }

    fn bind_udp<T: ToSocketAddrs>(bind_addr: &T) -> Result<UdpSocket, Error> {
        let sock = UdpSocket::bind(&bind_addr);
        match sock {
            Ok(s) => Ok(s),
            Err(_e) => {
                let new_bind = bind_addr.to_socket_addrs()?.next().unwrap().port() + 1;
                let ip = get_if_addrs().unwrap()[0].ip();
                let new_addr = SocketAddr::new(ip, new_bind);
                warn!("Binding local port failed, trying {}", new_addr);
                Connection::<RM>::bind_udp(&new_addr)
            },
        }
    }

    #[allow(dead_code)]
    pub fn callbackobj(&self) -> MutexGuard<Option<Arc<Callback<RM> + Send + Sync>>> { self.callbackobj.lock() }
}
