use std::{
    any::Any,
    collections::HashMap,
};

use snafu::{
    ensure,
    ResultExt,
    Snafu,
};

use crate::{
    is_empty_line::is_empty_line,
    parse_section::parse_section,
    ArrayParser,
};

#[derive(Snafu, Debug, PartialEq)]
pub enum SectionBuilderError {
    #[snafu(display("encountered an error while parsing the field {}: {}", field, source))]
    ArrayParserError {
        field: String,
        source: crate::ArrayParserError,
    },
    #[snafu(display("the following elements are duplicated: {:?}", duplicates))]
    ArrayWithDuplicates { duplicates: Vec<String> },
    #[snafu(display("field {} has not been closed", field))]
    ArrayNotClosed { field: String },
    #[snafu(display("{} field has already been set", field))]
    DuplicateField { field: String },
    #[snafu(display("{} is not a valid field", field))]
    InvalidField { field: String },
}

type Result<T, E = SectionBuilderError> = std::result::Result<T, E>;

fn add_field_value<T>(
    key: &str,
    value: T,
    values: &mut HashMap<&'static str, T>,
    fields: &'static [&'static str],
) -> Result<()> {
    ensure!(
        !values.contains_key(key),
        DuplicateField {
            field: key.to_owned()
        }
    );

    let map_key = fields.iter().find(|s| *s == &key);
    ensure!(
        map_key.is_some(),
        InvalidField {
            field: key.to_owned()
        }
    );

    values.insert(map_key.unwrap(), value);
    Ok(())
}

pub trait SectionBuilder: Any {
    fn section_name(&self) -> &'static str;
    fn get_fields(&self) -> &'static [&'static str];
    fn get_array_fields(&self) -> &'static [&'static str];
    fn get_code_fields(&self) -> &'static [&'static str];

    fn build(
        &mut self,
        values: &mut HashMap<&'static str, String>,
        array_values: &mut HashMap<&'static str, Vec<String>>,
    );

    fn parse_until_next_section<'a>(
        &mut self,
        lines: &'a [String],
    ) -> Result<&'a [String]> {
        let mut array_parser = ArrayParser::new();
        let mut values: HashMap<&'static str, String> = HashMap::new();
        let mut array_values: HashMap<&'static str, Vec<String>> = HashMap::new();
        let mut next_section: &'a [String] = &[];
        for (index, line) in lines.iter().enumerate() {
            let line = line.trim();
            if is_empty_line(line) {
                continue;
            } else if (array_parser.is_parsing && {
                array_parser.parse_line(line).context(ArrayParserError {
                    field: array_parser.key.to_owned(),
                })?;
                true
            }) || array_parser.start_parsing(line).context(ArrayParserError {
                field: array_parser.key.to_owned(),
            })? {
                if array_parser.is_parsing {
                    continue;
                }

                let key = array_parser.key.to_owned();
                add_field_value(
                    &key,
                    array_parser.get_values().context(ArrayParserError {
                        field: key.to_owned(),
                    })?,
                    &mut array_values,
                    self.get_array_fields(),
                )?;
                array_parser = ArrayParser::new();
            } else if parse_section(line).is_some() {
                next_section = &lines[index..];
                break;
            } else if let Some((key, value)) = line.split_once('=') {
                add_field_value(key, value.to_string(), &mut values, self.get_array_fields())?;
            }
        }

        // Check that all parsers state
        ensure!(
            !array_parser.is_parsing,
            ArrayNotClosed {
                field: array_parser.key
            }
        );

        self.build(&mut values, &mut array_values);

        Ok(next_section)
    }
}