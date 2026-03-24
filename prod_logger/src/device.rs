use tracing::{instrument, info};
use pcap::{Device, Capture, Active};

#[instrument]
pub fn get_pcap_capture() -> Result<Capture<Active>, &'static str> {
    // allocate a main device, probably not the one we want though
    let mut main_device = Device::lookup().unwrap().unwrap();
    // list all the available devices
    let devices = Device::list().unwrap();

    // iterate over each device, checking if we get a packet from the starsonata server
    let mut dev_found = false;
    for dev in devices.iter() {
        info!("Testing device {:?} {:?} ...", dev.name, dev.desc);
        // have to do this way so its inactive and we can set timeout
        let mut cl_cap = Capture::from_device(dev.clone()).unwrap();
        cl_cap = cl_cap.timeout(1000);

        let mut cap = cl_cap.open().unwrap();
        let _ = cap.filter("src host 51.222.248.34", true);  // ss live "liberty" server
        match cap.next_packet() {
            Ok(_) => {
                main_device = dev.clone();
                info!("Found Star Sonata packet from device: {:?}", dev.name);
                dev_found = true;
                break;
            }
            _ => {},
        };
    }
    if !dev_found {
        return Err("Unable to find device for capturing Star Sonata packets.");
    }

    // create the capture and open.
    let mut cap = Capture::from_device(main_device)
        .unwrap()
        // .promisc(true)
        .immediate_mode(true)
        .buffer_size(10000000)  // default: 1,000,000
        .snaplen(1000000)  // default: 65,535
        .open()
        // .unwrap()
        // .setnonblock()
        .unwrap();
    let _ = cap.filter(
        "src host 51.222.248.34", true
    );

    return Ok(cap)
}