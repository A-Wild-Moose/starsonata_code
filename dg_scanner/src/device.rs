use pcap::{Device, Capture, Active};

pub fn get_pcap_capture() -> Capture<Active> {
    // allocate a main device, probably not the one we want though
    let mut main_device = Device::lookup().unwrap().unwrap();
    // list all the available devices
    let devices = Device::list().unwrap();

    // iterate over each device, checking if we get a packet from the starsonata server
    for dev in devices.iter() {
        println!("\tTesting device {:?} ...", dev.desc);
        // have to do this way so its inactive and we can set timeout
        let mut cl_cap = Capture::from_device(dev.clone()).unwrap();
        cl_cap = cl_cap.timeout(1000);

        let mut cap = cl_cap.open().unwrap();
        let _ = cap.filter("src host 51.222.248.34", true);  // ss live "liberty" server
        match cap.next_packet() {
            Ok(_) => {
                main_device = dev.clone();
                break;
            }
            _ => (),
        };
    }

    // create the capture and open.
    let mut cap = Capture::from_device(main_device).unwrap().open().unwrap();
    let _ = cap.filter(
        "src host 51.222.248.34", true
    );

    return cap
}