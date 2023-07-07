use std::io::BufRead;

use hidapi::HidApi;


fn hexdump(buf: &[u8]) {
    let mut pos = 0;
    while pos < buf.len() {
        print!("{:08X} ", pos);
        for i in 0..16 {
            if pos + i < buf.len() {
                print!(" {:02X}", buf[pos + i]);
            } else {
                print!("   ");
            }
        }
        print!("  |");
        for i in 0..16 {
            if pos + i < buf.len() {
                let b = buf[pos + i];
                if b >= 0x20 && b <= 0x7E {
                    print!("{}", b as char);
                } else {
                    print!(".");
                }
            }
        }
        println!("|");
        pos += 16;
    }
}


fn main() {
    // assumes a Datalogic GD4500

    let hidapi = HidApi::new()
        .expect("failed to create HidApi");
    let device = hidapi.open(0x05F9, 0x322F)
        .expect("failed to open device");

    let mut report_descriptor = [0x00; hidapi::MAX_REPORT_DESCRIPTOR_SIZE];
    let report_length = device.get_report_descriptor(&mut report_descriptor)
        .expect("failed to read report descriptor");
    println!("report descriptor:");
    hexdump(&report_descriptor[..report_length]);

    // HID defines the message as:
    // [0] = report number (0x04 for the "Trigger Report")
    // [1] = bitfield:
    //       {1 << 0} = Power On Reset Scanner
    //       {1 << 1} = Prevent Read of Barcodes
    //       {1 << 2} = Initiate Barcode Read
    //       {1 << 3} = undefined
    //       {1 << 4} = Set Parameter Default Values
    //       {1 << 5} = Sound Error Beep
    //       {1 << 6} = Sound Good Beep
    //       {1 << 7} = Undefined

    let trigger_buf = [0x04, (1 << 1)];
    device.write(&trigger_buf)
        .expect("failed to write trigger report");

    println!("press Enter to enable read");
    {
        let stdin = std::io::stdin();
        let mut stdin_guard = stdin.lock();
        let mut buf = String::new();
        stdin_guard.read_line(&mut buf)
            .expect("failed to wait for Enter");
    }

    let trigger_buf = [0x04, 0x00];
    device.write(&trigger_buf)
        .expect("failed to write trigger report");

    println!("alright then, scan the barcode(s)");

    let mut barcode_buf: Vec<u8> = Vec::new();
    let mut msg_buf = [0u8; 4096];
    loop {
        let bytes_read = device.read(&mut msg_buf)
            .expect("failed to read data");
        let buf_slice = &msg_buf[..bytes_read];

        // HID defines the message as:
        // [0] = report number (0x02 for the "Scanned Data Report")
        // [1] = number of barcode data bytes in this message
        // [2..=4] = symbology identifier
        // [5..=60] = barcode data (only the first [number of barcode data bytes] are valid)
        // [61..=62] = some vendor-specific values, seem to generally be zeroes
        // [63] = 0x01 if the barcode data continues in the next message, 0x00 if we're done

        if buf_slice.len() != 64 {
            eprintln!("incorrect report length :-(");
            continue;
        }
        if buf_slice[0] != 0x02 {
            eprintln!("unexpected report 0x{:02X}; expected 0x02 :-(", buf_slice[0]);
            continue;
        }
        let data_byte_count: usize = buf_slice[1].into();
        if data_byte_count > 56 {
            eprintln!("invalid barcode data byte count (got {}, max is 56)", data_byte_count);
            continue;
        }
        if let Ok(symbo_ident) = std::str::from_utf8(&buf_slice[2..=4]) {
            eprintln!("symbology identifier: {}", symbo_ident);
        }
        barcode_buf.extend(&buf_slice[5..5+data_byte_count]);

        if buf_slice[63] == 0x00 {
            // barcode is complete
            hexdump(&barcode_buf);
            barcode_buf.clear();
        }
    }
}
