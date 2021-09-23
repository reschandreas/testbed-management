pub trait Preamble {
    fn get_variables(&self) -> Vec<(String, String, String)> {
        Vec::new()
    }
    fn get_packer_plugin(&self) -> String;
    fn get_values(&self) -> Vec<(&'static str, String)>;

    fn to_pkr_hcl(&self) -> String {
        let mut builder = string_builder::Builder::default();
        for (name, datatype, value) in self.get_variables() {
            builder.append("variable \"");
            builder.append(name);
            builder.append("\" {\n");
            builder.append("  type    = ");
            builder.append(datatype);
            builder.append("\n  default = \"");
            builder.append(value);
            builder.append("\"\n}\n");
        }
        builder.append("source \"");
        builder.append(self.get_packer_plugin());
        builder.append("\" \"imagefile\" {\n");
        for (key, value) in self.get_values() {
            crate::utils::add_indented_aligned_key_value(&mut builder, 2, 20, key, &value);
        }
        builder.string().unwrap()
    }
    /// # Errors
    ///
    /// Will return `Err` if `line` could not be parsed
    fn parse_base_image(&mut self, line: &str) -> Result<(), &'static str>;

    fn get_filename(&self) -> &str;

    fn set_filepath(&mut self, path: &str);

    #[must_use]
    fn get_checksum_type(&self) -> String;

    fn set_checksum(&mut self, checksum: String);

    fn get_preseed_file(&self) -> String;

    fn set_preseed_file(&mut self, path: String);
}
