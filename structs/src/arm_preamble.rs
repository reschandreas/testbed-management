use crate::preamble::Preamble;
use crate::utils;

#[derive(Debug)]
pub struct ArmPreamble {
    image_build_method: String,
    image_path: String,
    image_size: String,
    image_type: String,
    image_chroot_env: Vec<String>,
    file_checksum_type: String,
    file_checksum_url: String,
    file_target_extension: String,
    file_urls: Vec<String>,
}

impl ArmPreamble {
    #[must_use]
    pub fn new() -> Self {
        let mut vec = Vec::new();
        vec.push(String::from(
            "PATH=/usr/local/bin:/usr/local/sbin:/usr/bin:/usr/sbin:/bin:/sbin",
        ));
        ArmPreamble {
            image_build_method: String::from("reuse"),
            image_path: String::from("generated.img"),
            image_size: String::from("2G"),
            image_type: String::from("dos"),
            image_chroot_env: vec,
            file_checksum_type: String::new(),
            file_checksum_url: String::new(),
            file_target_extension: String::new(),
            file_urls: Vec::new(),
        }
    }
}

impl Default for ArmPreamble {
    fn default() -> Self {
        Self::new()
    }
}

impl Preamble for ArmPreamble {
    fn get_packer_plugin(&self) -> String {
        String::from("arm")
    }

    fn get_values(&self) -> Vec<(&'static str, String)> {
        let mut fields = Vec::new();
        fields.push(("image_build_method", utils::quote(&self.image_build_method)));
        fields.push(("image_path", utils::quote(&self.image_path)));
        fields.push(("image_size", utils::quote(&self.image_size)));
        fields.push(("image_type", utils::quote(&self.image_type)));
        fields.push((
            "image_chroot_env",
            utils::vec_to_string(&self.image_chroot_env, true),
        ));
        fields.push(("file_checksum_type", utils::quote(&self.file_checksum_type)));
        fields.push(("file_checksum_url", utils::quote(&self.file_checksum_url)));
        fields.push((
            "file_target_extension",
            utils::quote(&self.file_target_extension),
        ));
        fields.push(("file_urls", utils::vec_to_string(&self.file_urls, true)));
        fields
    }

    fn parse_base_image(&mut self, line: &str) -> Result<(), &'static str> {
        let mut file_checksum_type: String = String::from("sha256");
        let parts = line.split(' ').collect::<Vec<&str>>();
        if parts.len() < 2 {
            if parts.len() == 2 {
                file_checksum_type = String::from(parts[1]);
            }
            let mut file_urls = Vec::new();
            file_urls.push(String::from(parts[0]));
            let file_checksum_url = format!("{}.{}", parts[0], file_checksum_type);
            let file_target_extension = line.split('.').last().unwrap().to_string();
            self.file_checksum_type = file_checksum_type;
            self.file_checksum_url = file_checksum_url;
            self.file_target_extension = file_target_extension;
            self.file_urls = file_urls;
            Ok(())
        } else {
            Err("Could not parse Baseimage")
        }
    }

    #[must_use]
    fn get_filename(&self) -> &str {
        self.file_urls[0].as_str()
    }

    fn set_filepath(&mut self, path: &str) {
        self.file_urls[0] = path.to_string();
        self.file_target_extension = path.to_string().split('.').last().unwrap().to_string();
        self.file_checksum_url = format!("{}.{}", path, self.file_checksum_type);
    }

    #[must_use]
    fn get_checksum_type(&self) -> String {
        self.file_checksum_type.clone()
    }

    fn set_checksum(&mut self, checksum: String) {
        self.file_checksum_url = checksum;
    }

    fn get_preseed_file(&self) -> String {
        String::new()
    }

    fn set_preseed_file(&mut self, _path: String) {}
}
