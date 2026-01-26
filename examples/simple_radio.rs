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
    println!("Write command rf_cfg: {:02X?}", cmd);

    let cmd = radio.rf_cfg.read_command();
    println!("Read command rf_cfg: {:02X?}", cmd);

    // Setting a field using an enum.
    radio.rf09_cmd.value.set_cmd(TransceiverCmd::Sleep);

    let cmd = radio.rf09_cmd.write_command();
    println!("Write command rf09_cmd: {:02X?}", cmd);

    // Generating a read command for a large muli-register
    let cmd = radio.bbc0_cnt.read_command();
    println!("Read command bbc0_cnt: {:02X?}", cmd);

    // Bulk writes
    let mut pending_writes = BulkWrites::new();

    pending_writes.add(&mut radio.rf_cfg);
    pending_writes.add(&mut radio.rf09_cmd);
    pending_writes.add(&mut radio.rf_clko);

    // This should generate 2 write commands since rf_cfg and rf_clko are contiguous registers.
    let write_commands = pending_writes.generate_commands();
    for (idx, command) in write_commands.into_iter().enumerate() {
        println!("Bulk Write command [{}]: {:02X?}", idx, command)
    }
}
