extern crate lyon;

use sort_polygons::Polygon;

use gerber_types::*;

use std::io::Write;
use conv::TryFrom;

#[derive(Clone, Debug)]
pub struct GerberBuilder {
    cf: gerber_types::CoordinateFormat,
    commands: Vec<gerber_types::Command>,
}


const VERSION: &'static str = env!("CARGO_PKG_VERSION");


pub trait IntoCommand {
    fn into_command(self) -> gerber_types::Command;
}

impl IntoCommand for FunctionCode {
    fn into_command(self) -> gerber_types::Command {
        self.into()
    }
}

impl IntoCommand for gerber_types::GCode {
    fn into_command(self) -> gerber_types::Command {
        gerber_types::FunctionCode::GCode(self).into()
    }
}


impl IntoCommand for gerber_types::ExtendedCode {
    fn into_command(self) -> gerber_types::Command {
        self.into()
    }
}

impl IntoCommand for gerber_types::Operation {
    fn into_command(self) -> gerber_types::Command {
        gerber_types::FunctionCode::DCode(
            gerber_types::DCode::Operation(self)
        ).into()
    }
}


impl GerberBuilder {
    pub fn new(cf: gerber_types::CoordinateFormat, layer_type: Part, layer_func: FileFunction, file_polarity: bool) -> Self { GerberBuilder {
        cf: cf,
        commands: vec![

            // gerbv doesn't seem to parse these:
            ExtendedCode::FileAttribute(
                    FileAttribute::GenerationSoftware(
                        GenerationSoftware::new("SAM", "svg2gerber", Some(VERSION))
                        )
                    ).into(),

            ExtendedCode::FileAttribute(
                    FileAttribute::Part(layer_type)
                    ).into(),

            ExtendedCode::FileAttribute(
                    FileAttribute::FileFunction(layer_func)
                    ).into(),

            ExtendedCode::FileAttribute(
                    FileAttribute::FilePolarity(if file_polarity {FilePolarity::Positive} else {FilePolarity::Negative})
                    ).into(),

            //FunctionCode::GCode( GCode::Comment("Ucamco ex. 1: Two square boxes".to_string())).into(),
            ExtendedCode::CoordinateFormat(cf).into(),
            ExtendedCode::Unit(Unit::Millimeters).into(),

            // gerbv complains if there are no apertures defined
            ExtendedCode::ApertureDefinition(
                ApertureDefinition {
                    code: 10,
                    aperture: Aperture::Circle(Circle { diameter: 0.01, hole_diameter: None }),
                }
            ).into(),

            ExtendedCode::LoadPolarity(Polarity::Dark).into(), // this is the default, this makes our intentions explicit
            FunctionCode::GCode(
                GCode::InterpolationMode(InterpolationMode::Linear)
            ).into(),
        ],
    } }

    // this function should not require a mutable self, but then how can we call it in
    // a function that mutates self?
    pub fn vertex_to_coords(&self, vertex: &lyon::math::Point) -> Coordinates {
        // seriously?! this gerber library seems unnecessarily complicated
        Coordinates::new(
            CoordinateNumber::try_from( vertex.x as f64).unwrap(),
            // mirror vertical axis, as SVG and gerber use different conventions
            // TODO: this is a bit hackish, no? where shall we put this instead?
            CoordinateNumber::try_from(-vertex.y as f64).unwrap(),
            self.cf
        )
    }


    pub fn push<F: IntoCommand>(&mut self, cmd: F) {
        self.commands.push(cmd.into_command());
    }

    pub fn publish<W: Write>(&mut self, writer: &mut W) {
        // append EOF marker
        self.push(FunctionCode::MCode(MCode::EndOfFile));

        self.commands.serialize(writer).unwrap();
    }

    pub fn start_region(&mut self) {
        // start a new region
        self.push(GCode::RegionMode(true));
    }

    pub fn end_region(&mut self) {
        // end region
        self.push(GCode::RegionMode(false));
    }

    pub fn set_polarity(&mut self, polarity: bool) {
        if polarity { // true = positive = add
            self.push(ExtendedCode::LoadPolarity(Polarity::Dark));
        } else { // false = negative = clear
            self.push(ExtendedCode::LoadPolarity(Polarity::Clear));
        }
    }

    pub fn add_polygon(&mut self, poly: &Polygon) {
        self.start_region();

        // goto last point
        let pt = self.vertex_to_coords(poly.vertices.last().unwrap());
        self.push(gerber_types::Operation::Move(pt));

        // now convert the full polygon
        for ref v in &poly.vertices {
            let pt = self.vertex_to_coords(&v);
            self.push(gerber_types::Operation::Interpolate(pt, None));
        }

        self.end_region();
    }
}
