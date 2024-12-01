use std::io::Read;
use std::io::Write;
use std::mem::ManuallyDrop;
use std::num::NonZero;
use std::os::windows::io::OwnedHandle;
use std::sync::atomic;
use std::sync::atomic::AtomicBool;
use std::thread::JoinHandle;

use bytemuck::AnyBitPattern;
use bytemuck::NoUninit;

use crossbeam::channel::Receiver as MpscReceiver;
use crossbeam::channel::Sender as MpscSender;

use interprocess::unnamed_pipe::Recver as RecverU8;
use interprocess::unnamed_pipe::Sender as SenderU8;

pub fn bounded<T: Send + NoUninit + AnyBitPattern>(cap: usize) -> (Sender<T>, Receiver<T>) {
    let (sender, receiver) = interprocess::os::windows::unnamed_pipe::CreationOptions::new()
        .buffer_size_hint(NonZero::new(cap))
        .inheritable(true)
        .build()
        .unwrap();
    (Sender::new(sender), Receiver::new(receiver))
}

pub struct Sender<T: Send + NoUninit + AnyBitPattern> {
    sender: SenderU8,
    _phantom_data: std::marker::PhantomData<T>,
}

impl<T: Send + NoUninit + AnyBitPattern> Sender<T> {
    fn new(sender: SenderU8) -> Self {
        Self {
            sender,
            _phantom_data: Default::default(),
        }
    }

    pub fn send(&mut self, msg: T) -> Result<(), crossbeam::channel::SendError<T>> {
        self.sender
            .write_all(bytemuck::bytes_of(&msg))
            .map_err(|_| crossbeam::channel::SendError(msg))?;
        self.sender
            .flush()
            .map_err(|_| crossbeam::channel::SendError(msg))?;
        Ok(())
    }
}

impl<T: Send + NoUninit + AnyBitPattern> Into<OwnedHandle> for Sender<T> {
    fn into(self) -> OwnedHandle {
        self.sender.into()
    }
}

impl<T: Send + NoUninit + AnyBitPattern> From<OwnedHandle> for Sender<T> {
    fn from(value: OwnedHandle) -> Self {
        Self::new(value.into())
    }
}

pub struct Receiver<T: Send + NoUninit + AnyBitPattern> {
    receiver: RecverU8,
    buf: Box<[u8]>,
    _phantom_data: std::marker::PhantomData<T>,
}

impl<T: Send + NoUninit + AnyBitPattern> Receiver<T> {
    fn new(receiver: RecverU8) -> Self {
        Self {
            receiver,
            buf: (0..std::mem::size_of::<T>()).map(|_| 0).collect(),
            _phantom_data: Default::default(),
        }
    }

    pub fn recv(&mut self) -> std::io::Result<T> {
        self.receiver.read_exact(&mut self.buf)?;
        let msg = bytemuck::from_bytes(&self.buf);
        let msg = unsafe { std::ptr::read(msg as _) };
        Ok(msg)
    }
}

impl<T: Send + NoUninit + AnyBitPattern> Into<OwnedHandle> for Receiver<T> {
    fn into(self) -> OwnedHandle {
        self.receiver.into()
    }
}

impl<T: Send + NoUninit + AnyBitPattern> From<OwnedHandle> for Receiver<T> {
    fn from(value: OwnedHandle) -> Self {
        Self::new(value.into())
    }
}

pub struct NonBlockSender<T: Send + NoUninit + AnyBitPattern> {
    mpsc_sender: ManuallyDrop<MpscSender<T>>,
    is_running: std::sync::Arc<AtomicBool>,
    thread: ManuallyDrop<JoinHandle<()>>,
}

impl<T: Send + NoUninit + AnyBitPattern> NonBlockSender<T> {
    pub fn bounded(mut sender: Sender<T>, cap: usize) -> Self {
        let (mpsc_sender, mpsc_receiver) = crossbeam::channel::bounded(cap);
        let is_running = std::sync::Arc::new(AtomicBool::new(true));
        let thread_is_running = is_running.clone();
        let thread = std::thread::spawn(move || {
            let is_running = thread_is_running;
            while is_running.load(atomic::Ordering::Relaxed) {
                for msg in mpsc_receiver.iter() {
                    let Ok(_) = sender.send(msg) else {
                        #[cfg(debug_assertions)]
                        println!("NonBlockSender thread send failed");

                        return;
                    };
                }
            }
        });
        Self {
            mpsc_sender: ManuallyDrop::new(mpsc_sender),
            is_running,
            thread: ManuallyDrop::new(thread),
        }
    }

    pub fn send(&self, msg: T) -> Result<(), crossbeam::channel::SendError<T>> {
        self.mpsc_sender.send(msg)
    }
}

impl<T: Send + NoUninit + AnyBitPattern> Drop for NonBlockSender<T> {
    fn drop(&mut self) {
        self.is_running.store(false, atomic::Ordering::Relaxed);
        unsafe { ManuallyDrop::drop(&mut self.mpsc_sender) };
        unsafe { ManuallyDrop::take(&mut self.thread) }
            .join()
            .unwrap();
    }
}

pub struct NonBlockReceiver<T: Send + NoUninit + AnyBitPattern> {
    mpsc_receiver: ManuallyDrop<MpscReceiver<T>>,
    is_running: std::sync::Arc<AtomicBool>,
    thread: ManuallyDrop<JoinHandle<()>>,
}

impl<T: Send + NoUninit + AnyBitPattern> NonBlockReceiver<T> {
    pub fn bounded(mut receiver: Receiver<T>, cap: usize) -> Self {
        let (mpsc_sender, mpsc_receiver) = crossbeam::channel::bounded(cap);
        let is_running = std::sync::Arc::new(AtomicBool::new(true));
        let thread_is_running = is_running.clone();
        let thread = std::thread::spawn(move || {
            let is_running = thread_is_running;
            while is_running.load(atomic::Ordering::Relaxed) {
                let Ok(msg) = receiver.recv() else {
                    #[cfg(debug_assertions)]
                    println!("NonBlockReceiver thread recv failed");

                    return;
                };
                let _ = mpsc_sender.send(msg);
            }
        });
        Self {
            mpsc_receiver: ManuallyDrop::new(mpsc_receiver),
            is_running,
            thread: ManuallyDrop::new(thread),
        }
    }

    pub fn try_iter(&self) -> crossbeam::channel::TryIter<'_, T> {
        self.mpsc_receiver.try_iter()
    }
}

impl<T: Send + NoUninit + AnyBitPattern> Drop for NonBlockReceiver<T> {
    fn drop(&mut self) {
        self.is_running.store(false, atomic::Ordering::Relaxed);
        unsafe { ManuallyDrop::drop(&mut self.mpsc_receiver) };
        unsafe { ManuallyDrop::take(&mut self.thread) }
            .join()
            .unwrap();
    }
}
