#[macro_use] extern crate log;
extern crate env_logger;

extern crate usvg;
extern crate gerber_types;
extern crate lyon;
extern crate conv;

mod path_convert;
mod gerber_builder;
mod sort_polygons;

use std::path::Path;
use std::fs::File;
use std::io::stdout;
use std::env;

use gerber_types::*;

//use lyon::tessellation as tess;
use lyon::path::iterator::PathIterator;


fn main() {
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    if args.len() != 2 && args.len() != 3 && args.len() != 4 {
        println!("Usage:\n\tsvg2gerber input.svg [output.gerb [layer_type]]");
        return;
    }

    // load svg from disk and simplify it considerably with usvg
    let svg = usvg::Tree::from_file(&args[1], &usvg::Options::default()).expect("failed to load SVG");

    // TODO: for a proper script we probably want to add separate parameters for all of these:
    let part_type = if args.len() < 4 {
        (gerber_types::Part::Other("Unknown".to_string()), gerber_types::FileFunction::Other("Unkown".to_string()), true)
    } else {
        match args[3].to_ascii_lowercase().as_ref() {
            "f.cu"   => (gerber_types::Part::Single, gerber_types::FileFunction::Copper{layer: 1, pos: gerber_types::ExtendedPosition::Top,    copper_type: None}, true),
            "b.cu"   => (gerber_types::Part::Single, gerber_types::FileFunction::Copper{layer: 2, pos: gerber_types::ExtendedPosition::Bottom, copper_type: None}, true),
            "f.mask" => (gerber_types::Part::Single, gerber_types::FileFunction::Soldermask{index: None, pos: gerber_types::Position::Top},    false),
            "b.mask" => (gerber_types::Part::Single, gerber_types::FileFunction::Soldermask{index: None, pos: gerber_types::Position::Bottom}, false),
            &_ => panic!("unknown layer type {}", args[3]),
        }
    };


    let mut gerb = gerber_builder::GerberBuilder::new(
        CoordinateFormat::new(5, 6),
        part_type.0, // part type (single PCB or 'other')
        part_type.1, // file function (copper, solder mask, ...)
        part_type.2, // polarity (true = positive = add stuff where gerber has stuff)
    );

    //let mut fill_tess = lyon::tessellation::FillTessellator::new();

    let mut polys = Vec::new();
    for node in svg.root().descendants() {
        if let usvg::NodeKind::Path(ref p) = *node.borrow() {

            // TODO: do we have to handle transformations here? usvg should already have removed
            // those for us, no?
            let path = path_convert::convert_path(p).path_iter();

            // convert path containing bezier curves and other complicated things into something
            // piece-wise linear
            let flattened = path.flattened(0.01);

            polys.extend(sort_polygons::Polygon::from_path(flattened));

            /*
            fill_tess.tessellate_path(
                path_convert::convert_path(p).path_iter(),
                &lyon::tessellation::FillOptions::tolerance(0.01),
                &mut data
            ).expect("Error during tesselation!");
            */
        }
        // TODO: extract layer information from Inkscape-SVG
    }

    let mut parents = sort_polygons::create_parent_list(&polys);

    info!("got {} polygons", polys.len());

    parents.sort_unstable_by_key(|pi| pi.level);

    for (idx, p) in parents.iter().enumerate() {
        info!(" - poly {} has parent {:?} and level {}", idx, p.parent_idx, p.level);

        gerb.set_polarity(p.level % 2 == 0);
        gerb.add_polygon(p.polygon);
    }

    if args.len() > 2 {
        if args[2] == "-" {
            // publish to stdout
            gerb.publish(&mut stdout());
        } else {
            let mut outfile = File::create(&args[2]).expect("Could not create output file");
            gerb.publish(&mut outfile);
        }
    } else {
        let path = Path::new(&args[1]);
        let path = Path::new(path.file_stem().unwrap()).with_extension("gerb");
        let mut outfile = File::create(path).expect("Could not create output file");
        gerb.publish(&mut outfile);
    }
}
