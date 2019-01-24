extern crate lyon;

//extern crate gnuplot;
//use sort_polygons::gnuplot::AxesCommon;

//use lyon::tessellation as tess;
use lyon::path::iterator::PathIterator;
use lyon::path::FlattenedEvent;
use lyon::algorithms::path::iterator::Flattened;

#[derive(PartialEq)]
pub struct Polygon {
    pub vertices: Vec<lyon::math::Point>,
}

impl Polygon {
    pub fn new() -> Self {
        Polygon {
            vertices: Vec::new(),
        }
    }

    pub fn from_path<Iter: PathIterator>(path: Flattened<Iter>) -> Vec<Self> {
        let mut polys = Vec::new();
        let mut current_poly = None;

        for evt in path {
            match evt {
                FlattenedEvent::MoveTo(p) => {
                    let mut poly = Polygon::new();
                    poly.vertices.push(p);
                    current_poly = Some(poly);
                }

                FlattenedEvent::LineTo(p) => {
                    if let Some(ref mut poly) = current_poly {
                        poly.vertices.push(p);
                    }
                }

                FlattenedEvent::Close => {
                    polys.push(current_poly.unwrap());
                    current_poly = None;
                }
            }
        }

        polys
    }
}


pub struct PolyPoint<'a> {
    vertex: &'a lyon::math::Point,
    prev:   &'a lyon::math::Point,
    next:   &'a lyon::math::Point,

    //pub poly_parent: &'a mut Option<&'a Polygon<'a>>,
    poly_idx: usize,

    // TODO: equality operator that just compares 'vertex'
}

impl<'a> PolyPoint<'a> {
    pub fn list(polys: &'a Vec<Polygon>) -> Vec<PolyPoint> {
        let mut pts = Vec::new();

        for (poly_idx, poly) in polys.iter().enumerate() {

            assert!(poly.vertices.len() >= 3, "Got a degenerate polygon with only {} vertices.", poly.vertices.len());

            let mut v1 = &poly.vertices[poly.vertices.len()-2]; // second last element
            let mut v2 = &poly.vertices[poly.vertices.len()-1]; // last element

            debug_assert!(v1 != v2, "got same element twice: point {}", v1);

            for v3 in &poly.vertices {
                pts.push(PolyPoint {

                    prev:   v1,
                    vertex: v2,
                    next:   v3,

                    poly_idx: poly_idx,
                });

                //trace!("adding Vertex: {}, {}, {}", v1, v2, v3);

                v1 = v2;
                v2 = v3;
            }
        }

        pts
    }
}

// returns points sorted in descending y order
pub fn sort_poly_points(pts: &mut Vec<PolyPoint>) {
    // sorting floats is only possible when we don't have NaNs
    pts.sort_by(|a, b| b.vertex.y.partial_cmp(&a.vertex.y).unwrap() );
}


struct Edge<'a> {
    upper: &'a lyon::math::Point, // higher y
    lower: &'a lyon::math::Point, // lower y

    poly_idx: usize,
}

impl<'a> Edge<'a> {
    pub fn interpolate_x(&self, y: f32) -> f32 {
        debug_assert!(self.upper.y >= y && y >= self.lower.y,
                      "interpolation point must lie between edge's end points: Edge is from {} to {}, query y is {}.", self.upper.y, self.lower.y, y);

        let    r = (y - self.lower.y) / (self.upper.y - self.lower.y);
        return r * (self.upper.x - self.lower.x) + self.lower.x;
    }
}


// insert or remove edge from scanline (scanline is at 'vert')
fn handle_edge<'a>(scanline: &mut Vec<Edge<'a>>, vert: &'a lyon::math::Point, other: &'a lyon::math::Point, poly_idx: usize) {
    if vert == other {
        return; // ignore degenerate edges with zero length
    }
    //debug_assert!(vert != other, "Edge must consist of two distinct points, but got {} twice.", vert);
    trace!(" -> handling edge from {} to {} (poly {})", vert, other, poly_idx);

    if other.y == vert.y {
        trace!("     -> ignoring horizontal edges");
        return;
    }

    if other.y > vert.y {
        // edge ends at scanline
        // remove it from scanline
        // TODO: implement ordering trait for Edge that uses interpolated x value so we can find
        // our edge more efficiently
        trace!("     -> removing edge, it ends here");
        scanline.retain(|edge| edge.lower != vert);
    } else {
        // edge starts at scanline
        // insert it in a sorted fashion
        let index;
        match scanline.binary_search_by(|edge| {edge.interpolate_x(vert.y).partial_cmp(&vert.x).unwrap()}) {
            Ok(i)  => index = i, // found other edge at this point. TODO: This should not happen and we probably want at least a warning here.
            Err(i) => index = i, // not found, but it belongs there
        }
        trace!("     -> insert edge at index {}", index);
        scanline.insert(index, Edge { upper: vert, lower: other, poly_idx: poly_idx });
    }
}

#[derive(Clone)]
pub struct ParentInfo<'a> {
    pub polygon: &'a Polygon,
    pub parent_idx: Option<usize>,
    pub level: usize, // 0 means poly is outermost
}

pub fn create_parent_list<'a>(polygons: &'a Vec<Polygon>) -> Vec<ParentInfo<'a>> {
    let mut pts = PolyPoint::list(&polygons);
    sort_poly_points(&mut pts);

    let mut current_scanline: Vec<Edge> = Vec::new();
    let mut parents: Vec<Option<ParentInfo>> = vec![None; polygons.len()];

    for (_step, pt) in pts.iter().enumerate() {
        trace!("scanline is at y = {}", pt.vertex.y);
        // look at edge (prev, vertex)
        handle_edge(&mut current_scanline, pt.vertex, pt.prev, pt.poly_idx);

        // look at edge (vertex, next)
        handle_edge(&mut current_scanline, pt.vertex, pt.next, pt.poly_idx);

        let mut parent_stack: Vec<usize> = Vec::new();



        /*
        let mut fig = gnuplot::Figure::new();
        {
        let mut ax = fig.axes2d();

        ax.set_title(&format!("Step {}", _step), &[]);
        ax.lines(&[pt.vertex.x, pt.prev.x], &[pt.vertex.y, pt.prev.y], &[gnuplot::Color("black"), gnuplot::LineWidth(2.0)]);
        ax.lines(&[pt.vertex.x, pt.next.x], &[pt.vertex.y, pt.next.y], &[gnuplot::Color("black"), gnuplot::LineWidth(2.0)]);
        ax.points(&[pt.vertex.x], &[pt.vertex.y], &[gnuplot::Color("black"), gnuplot::PointSize(5.0), gnuplot::PointSymbol('o')]);
        */

        // count number of edges between current vertex and the outside (while ignoring edges of
        // the current polygon)
        for ref edge in &current_scanline {

            // only look at edges on the left of the current vertex
            if edge.interpolate_x(pt.vertex.y) >= pt.vertex.x {
                break;
            }

            // ignore edges from current polygon
            if edge.poly_idx == pt.poly_idx {
                continue;
            }

            //ax.lines(&[edge.upper.x, edge.lower.x], &[edge.upper.y, edge.lower.y], &[gnuplot::Color("red"), gnuplot::LineWidth(2.0)]);

            // push or pop polys to/from stack
            let mut pop = false;
            if let Some(p) = parent_stack.last() {
                if *p == edge.poly_idx {
                    pop = true;
                }
            }

            if pop {
                parent_stack.pop();
            } else {
                parent_stack.push(edge.poly_idx);
            }
        }

        /*
        }
        fig.show();
        */

        trace!(" -> handling point {:?}", pt.vertex);
        trace!("    -> last edge on stack of {}: {:?}", parent_stack.len(), parent_stack.last());

        if !parents[pt.poly_idx].is_some() {
            // parent information not yet defined, add it
            parents[pt.poly_idx] = Some(ParentInfo{
                polygon: &polygons[pt.poly_idx],
                parent_idx: parent_stack.last().cloned(),
                level     : parent_stack.len(),
            });
            trace!("    -> assigned parent {:?}", parent_stack.last());
        } else if let Some(ref pi) = &parents[pt.poly_idx] {
            // polygon at poly_idx already has a parent & level defined
            // make sure it is the right one
            // (this should not be necessary, just to make sure our implementation is correct
            // and our assumptions were valid)

            assert!(pi.level == parent_stack.len(),
            "Invalid level for polygon {}: Expected {}, but we previously calculated {}.",
            pt.poly_idx, parent_stack.len(), pi.level);

            assert!(pi.parent_idx.is_some() == parent_stack.last().is_some(),
            "Invalid parent for polygon {}: Expected to have parent? {}. Previously determined: {}.",
            pt.poly_idx, parent_stack.last().is_some(), pi.parent_idx.is_some());

            if let Some(p) = pi.parent_idx {
                assert!(p == *parent_stack.last().unwrap(),
                "Invalid parent computed: Polygon {} already has parent {}, but we just found {:?} as parent.",
                pt.poly_idx, p, parent_stack.last());
            }
        }
    }

    assert!(parents.len() == polygons.len(), "Did not process all polygons. Only got {} out of {}.", parents.len(), polygons.len());

    parents.into_iter().map(|p| p.unwrap()).collect()
}
