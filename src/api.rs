//! Prototype of the Aves system calls.
//! 
//! The goal of Aves is to provide a "everything is a file", or
//! exactly, everything is a virtual file. This is more advanced
//! than linux because sockets are also represented in the file
//! system. This allows to generalize the concept of "block 
//! device" and virtual file systems.


extern {
    fn open(path: &str, options: &str) -> usize;
    fn free(handle: usize);
}


unsafe fn test() {
    
    // Open a TCP socket connected to 215.98.166.36:9832
    // The two following calls are equivalent, the second
    //  can be used for libraries to avoid encoding the
    //  ip address into a decimal numbers or variable
    //  length.
    open("/sys/ip4/126.98.166.36/tcp/9832", "r");
    open("/sys/ip4/x7E62A624/tcp/x2668", "r"); // -> 0x00000005
    // This would be implemented by multiple "fs drivers".
    // In fact, the IPv4 driver will provide an abstraction
    // for the TCP and UDP drivers and will need lower-level
    // drivers, typically to talk to network hardware.

    // The returned handle can be accessed later.
    open("/proc/self/io/x00000005/ip", "");  // -> ip4

    // Listen on a port.
    open("/sys/ip4/0.0.0.0/tcp/22", "l");

    // In the following snippet, we typically use the 
    // "network" fs driver (providing /dev/net/ directory).
    // This driver uses lower-level drivers specific to
    // hardware cards to register the interfaces.
    open("/sys/net/eth0/ratelimit", "rw");

    // Obvious...
    open("/home/me/ok.txt", "w");

    // Open the random device.
    open("/sys/rand", "r");

    // Resolve the hostname and return a handle that will 
    // just return the ip and its type (v4 or v6).
    open("/sys/hostname/google.fr", "r");

    // Create a custom host that can later be resolved.
    // The issue here is that it's not persistent, we could 
    // later add a file like /conf/kernel for example that 
    // can automatically configure the whole system at startup.
    open("/sys/hostname/customhost", "w");

}


// Suchz