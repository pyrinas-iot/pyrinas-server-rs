// Lib related
pub mod broker;
pub mod bucket;
pub mod influx;
pub mod mqtt;
pub mod ota_db;
pub mod settings;
pub mod sock;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
