#!/usr/bin/python3

import xml.etree.ElementTree as ET
import glob
import os.path as path
import os
import subprocess

UI_PATH = "ui/*.ui"

SRC_PATH = "../src/ui/builder.rs"


def main():
    global UI_PATH, SRC_PATH

    os.chdir("data")
    ui_files = glob.glob(UI_PATH)
    ui_files.sort()

    with open(SRC_PATH, "w") as src:
        first = True
        for file in ui_files:
            filename, _ = path.splitext(path.basename(file))
            rs_type = ''.join(x.title() for x in filename.split('_'))
            if first:
                first = False
            else:
                src.write("\n\n")

            src.write(struct_code(rs_type, file))

        src.write("\n")

    subprocess.call(["cargo", "fmt", "--", SRC_PATH])


class Item:
    def __init__(self, id, crate, type):
        self.id = id
        self.crate = crate
        self.type = type


def objects(path):
    objects = []
    for item in ET.parse(path).iter():
        if item.tag == "object" and item.get("id"):
            if item.get("class")[:3] == "Hdy":
                crate = "libhandy"
            else:
                crate = item.get("class")[:3].lower()

            objects.append(Item(item.get("id"), crate, item.get("class")[3:]))
    objects.sort(key=lambda item: item.id)

    return objects


def fn_code(objects):
    template = """
    pub fn {id}(&self) -> {crate}::{type} {{
        self.get("{id}")
    }}"""

    code = ""
    for o in objects:
        if code:
            code += "\n"
        code += template.format(**o.__dict__)

    return code


def struct_code(name, path):
    template = """pub struct {name} {{
    builder: gtk::Builder,
}}

impl {name} {{
    pub fn new() -> Self {{
        Self {{
            builder: gtk::Builder::from_string(include_str!(concat!(
                data_dir!(),
                "/{path}"
            ))),
        }}
    }}

    fn get<T: gtk::glib::IsA<gtk::glib::object::Object>>(&self, id: &str) -> T {{
        gtk::prelude::BuilderExtManual::get_object(&self.builder, id)
            .unwrap_or_else(|| panic!("Object with id '{{}}' not found in '{path}'", id))
    }}
{fn_code}
}}"""

    code = fn_code(objects(path))
    return template.format(name=name, path=path, fn_code=code)


if __name__ == "__main__":
    main()

