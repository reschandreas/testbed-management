use crate::preamble::Preamble;
use crate::utils;
use crate::utils::get_random_name;

#[derive(Debug)]
pub struct X86Preamble {
    pub boot_command: Vec<String>,
    pub boot_wait: String,
    pub disk_size: usize,
    pub guest_additions_mode: String,
    pub guest_os_type: String,
    pub http_directory: String,
    pub headless: bool,
    pub iso_checksum: String,
    pub iso_checksum_type: String,
    pub iso_url: String,
    pub shutdown_command: String,
    pub ssh_username: String,
    pub ssh_password: String,
    pub ssh_wait_timeout: String,
    pub vm_name: String,
    pub preseed_file: String,
    pub output_directory: String,
}

impl X86Preamble {
    #[must_use]
    pub fn new() -> Self {
        X86Preamble {
            boot_command: Vec::new(),
            boot_wait: String::from("30s"),
            disk_size: 8192,
            guest_additions_mode: String::from("disable"),
            guest_os_type: String::from("Linux26_64"),
            headless: true,
            http_directory: String::from("http"),
            iso_checksum: String::new(),
            iso_checksum_type: String::from("sha256"),
            iso_url: String::new(),
            shutdown_command: String::from("echo 'vagrant' | poweroff"),
            ssh_username: String::from("root"),
            ssh_password: String::from("alpine"),
            ssh_wait_timeout: String::from("1000s"),
            vm_name: get_random_name(),
            preseed_file: String::new(),
            output_directory: String::from("output"),
        }
    }

    pub fn set_ssh_username(&mut self, str: String) {
        self.ssh_username = str;
    }

    pub fn set_ssh_password(&mut self, str: String) {
        self.ssh_password = str;
    }

    pub fn set_boot_wait(&mut self, str: String) {
        self.boot_wait = str;
    }

    pub fn set_headless(&mut self, value: bool) {
        self.headless = value;
    }

    pub fn set_shutdown_command(&mut self, str: String) {
        self.shutdown_command = str;
    }

    pub fn set_guest_os_type(&mut self, str: String) {
        self.guest_os_type = str;
    }
}

impl Default for X86Preamble {
    fn default() -> Self {
        Self::new()
    }
}

impl Preamble for X86Preamble {
    fn get_variables(&self) -> Vec<(String, String, String)> {
        let mut vec = Vec::new();
        vec.push((
            String::from("vmname"),
            String::from("string"),
            self.vm_name.clone(),
        ));
        vec
    }

    fn get_packer_plugin(&self) -> String {
        String::from("virtualbox-iso")
    }

    fn get_values(&self) -> Vec<(&'static str, String)> {
        let mut fields = Vec::new();
        fields.push((
            "boot_command",
            utils::vec_to_string(&self.boot_command, true),
        ));
        fields.push(("boot_wait", utils::quote(&self.boot_wait)));
        fields.push(("disk_size", self.disk_size.to_string()));
        fields.push((
            "guest_additions_mode",
            utils::quote(&self.guest_additions_mode),
        ));
        fields.push(("guest_os_type", utils::quote(&self.guest_os_type)));
        fields.push(("headless", self.headless.to_string()));
        if !self.preseed_file.is_empty() {
            fields.push(("http_directory", utils::quote(&self.http_directory)));
        }
        fields.push((
            "iso_checksum",
            utils::quote(&format!(
                "{}:{}",
                &self.iso_checksum_type, &self.iso_checksum
            )),
        ));
        fields.push(("iso_url", utils::quote(&self.iso_url)));
        fields.push(("shutdown_command", utils::quote(&self.shutdown_command)));
        fields.push(("ssh_password", utils::quote(&self.ssh_password)));
        fields.push(("ssh_username", utils::quote(&self.ssh_username)));
        fields.push(("ssh_wait_timeout", utils::quote(&self.ssh_wait_timeout)));
        fields.push(("vm_name", utils::quote(&self.vm_name)));
        fields.push(("output_directory", utils::quote(&self.output_directory)));
        fields
    }

    fn parse_base_image(&mut self, line: &str) -> Result<(), &'static str> {
        let mut iso_checksum_type: String = String::from("sha256");
        let parts = line.split(' ').collect::<Vec<&str>>();
        if parts.len() <= 2 {
            if parts.len() == 2 {
                iso_checksum_type = String::from(parts[1]);
            }
            self.iso_url = String::from(parts[0]);
            self.iso_checksum_type = iso_checksum_type;
            self.iso_checksum = "".to_string();
            return Ok(());
        }
        Err("Could not parse Baseimage")
    }

    #[must_use]
    fn get_filename(&self) -> &str {
        self.iso_url.as_str()
    }

    fn set_filepath(&mut self, path: &str) {
        self.iso_url = path.to_string();
    }

    #[must_use]
    fn get_checksum_type(&self) -> String {
        self.iso_checksum_type.clone()
    }

    fn set_checksum(&mut self, checksum: String) {
        self.iso_checksum = checksum;
    }

    fn get_preseed_file(&self) -> String {
        self.preseed_file.clone()
    }

    fn set_preseed_file(&mut self, path: String) {
        self.preseed_file = path;
    }
}
