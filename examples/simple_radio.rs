use oresat_at86rf215_driver::radio::*;
use oresat_at86rf215_driver::registers::*;

fn main() {
    // Create an simple radio
    let mut radio = Radio::new();

    // Setting multiple values
    radio.rf_cfg.value = radio.rf_cfg.value.with_drv(3).with_irqmm(true);

    // Setting a single value
    radio.rf_cfg.value.set_irqp(true);

    let cmd = radio.rf_cfg.write_command();
    println!("Write command: {:02X?}", cmd);

    let cmd = radio.rf_cfg.read_command();
    println!("Read command: {:02X?}", cmd);

    // Example of setting a field using an enum.
    radio.rf09_cmd.value.set_cmd(TransceiverCmd::Sleep);

    let cmd = radio.rf09_cmd.write_command();
    println!("Write command: {:02X?}", cmd);

    // Exmple of generating a read command for a large muli-register
    let cmd = radio.bbc0_cnt.read_command();
    println!("Read command: {:02X?}", cmd)
}
