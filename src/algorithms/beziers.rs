use adaptive_bezier::adaptive_bezier_curve;

use crate::errors::CommandError;
use crate::utils::point::Point;

struct PolyBezier {
    vertices: Vec<Point>,
    controls: Vec<(Point,Point)> // this should always have one less item than vertices.
}

impl PolyBezier {

    #[cfg(test)] pub(crate) fn segment_at(&self, index: usize) -> Option<(&Point,&Point,&Point,&Point)> {
        if index < self.controls.len() {
            let v1 = &self.vertices[index];
            let c = &self.controls[index];
            let v2 = &self.vertices[index + 1];
            Some((v1,&c.0,&c.1,v2))
        } else {
            None
        }
    }

    pub(crate) fn trim_end(self) -> Self {
        let vertices_take = self.vertices.len() - 1;
        let controls_take = self.controls.len() - 1;
        Self {
            vertices: self.vertices.into_iter().take(vertices_take).collect(),
            controls: self.controls.into_iter().take(controls_take).collect(),
        }
    }

    pub(crate) fn trim_start(self) -> Self {
        Self {
            vertices: self.vertices.into_iter().skip(1).collect(),
            controls: self.controls.into_iter().skip(1).collect(),
        }
    }

    pub(crate) fn trim_both(self) -> Self {
        let vertices_take = self.vertices.len() - 1;
        let controls_take = self.controls.len() - 1;
        Self {
            vertices: self.vertices.into_iter().take(vertices_take).skip(1).collect(),
            controls: self.controls.into_iter().take(controls_take).skip(1).collect(),
        }
    }

    // finds a curve from a line where the first points and last points are curved with influence of optional extended points.
    // The curves created by these end segments are not included in the result.
    pub(crate) fn from_poly_line_with_phantoms(phantom_start: Option<&Point>, line: &[Point], phantom_end: Option<&Point>) -> Self {
        match (phantom_start,phantom_end) {
            (None, None) => Self::from_poly_line(line),
            (None, Some(end)) => {
                let mut vertices = line.to_vec();
                vertices.push(end.clone());
                let result = Self::from_poly_line(&vertices);
                result.trim_end()
            },
            (Some(start), None) => {
                let mut vertices = vec![start.clone()];
                vertices.extend(line.iter().cloned());
                let result = Self::from_poly_line(&vertices);
                result.trim_start()
            },
            (Some(start), Some(end)) => {
                let mut vertices = vec![start.clone()];
                vertices.extend(line.iter().cloned());
                vertices.push(end.clone());
                let result = Self::from_poly_line(&vertices);
                result.trim_both()
            },
        }

    }

    pub(crate) fn from_poly_line(vertices: &[Point]) -> Self {
        if vertices.len() < 2 {
            return Self {
                vertices: vertices.to_vec(),
                controls: Vec::new()
            }
        }

        // https://math.stackexchange.com/a/4207568
        /*
    STORY: I had a little artifical help from chatgpt to get the initial translation from python code in 
    the SO answer to rust. As an experience, it was like getting help from an idiot who is good at programming 
    and thinks he's an expert. The initial result looked like real code, and seemed to be doing what it
    was supposed to. 

    But, I would report compilation errors to it and it would say "Oh, sorry about that. This will compile." 
    Except it didn't. Every time it was confidently incorrect.

    It missed out on what was going on. For some reason, the initial translation required the input to be a vector
    of tuples of points, which didn't make sense. At one point it got into a cycle where it decided to fix things 
    by turning the points into tuples, then turning those into points, then back into tuples.

    I finally got the best results by starting over with a new conversation. Then I took the original code from
    stackoverflow, removed all of the plotting stuff to remove confusion, and told it to straight up translate that.
    I then used the engine linked to in the stackoverflow comment to change the code to print out the results, so 
    I could compare, and they were way off.

    I discovered two mistakes I never would have known about if I didn't look through the code carefully. It was like 
    chat decided that one operation was as good as another. The first was how it decided what to add to the start and
    end when the line wasn't a ring. The second was the call to get the absolute value of the point (`vertex.subtract(vertex0).abs()`).

    Even though it had figured out point subtraction, addition and multiplication, it decided that that the original
    code (`abs(p - p0)`) meant to take the absolute values of x and y and add them together. I searched for what it meant
    to get the absolute value of a point, and learned it was the distance from 0. Which meant chat decided that adding
    the values together was the same as adding their squares and then returning the square root.

    What if the difference between real intelligence and artificial intelligence is understanding the pythagorean theorem? What
    if Pythagoras was the person who invented human intelligence?

    The final result got me to almost match the values returned from the python code. The only differences were in the last digits
    and the number of digits returned, so it was just a matter of precision.
    */

        // Make the normalized tangent vectors
    
        // Tangents for interior points are parallel to the lines between the points to either side 
        // (tangent for point B is parallel to the line between A and B), so we need to pair up
        // the vertices as p,p+2. This will create n-2 vertices to match up with interior points.
        let pairs = vertices.iter().zip(vertices.iter().skip(2));
        // tangents for these pairs are found by subtracting the points
        let tangents: Vec<Point> = pairs.map(|(u, v)| v.subtract(u)).collect();

        // the start and end tangents are from different pairs.
        let (start,end) = if vertices[0] == vertices[vertices.len() - 1] {
            // this is a polygonal ring, so the points are the same, and the tangents for
            // them are the same. This tangent is parallel to a line from the second point to the penultimate point.
            // ABCDEA => paralell to BE
            // No panic, because we checked for vertices < 2 above.
            let end = vec![vertices[1].subtract(&vertices[vertices.len() - 2])];
            (end.clone(),end)
        } else {
            // otherwise, the start tangent is parallel to a line between the first and second point,
            // and the end tangent the same between the last and penultimate point.
            // ABCDE => parallel to AB and DE
            // start is the difference between the second and first
            let start = vec![vertices[1].subtract(&vertices[0])];
            // end is the difference between the last and second-to-last
            // No panic, because we checked for vertices < 2 above.
            let end = vec![vertices[vertices.len()-1].subtract(&vertices[vertices.len()-2])];
            (start,end)
        };

        let tangents = start.iter().chain(tangents.iter()).chain(end.iter());
        // the tangents are normalized -- we just need the direction, not the distance, so this is a unit vector pointing the same direction.
        let tangents = tangents.map(Point::normalized);
        let tangents: Vec<Point> = tangents.collect();

        // Build Bezier curves
        // zip up the points into pairs with their tangents
        let mut vertex_tangents = vertices.iter().zip(tangents.iter());
        // the first one should always be there? 
        // No panic, because we checked for vertices < 2 above.
        let (mut vertex0, mut tangent0) = vertex_tangents.next().expect("This shouldn't happeen because we checked if vertices < 2.");
        let mut controls = Vec::new();
        for (vertex, tangent) in vertex_tangents {
            // original code: s = abs(p - p0) / 3 
            let s = vertex.subtract(vertex0).abs() / 3.0;
            controls.push((
                // control point from previous point, on its tangent, 1/3 along the way between the two points
                vertex0.add(&tangent0.multiply(s)),
                // control point for the next point, on its tangent, 1/3 along the way
                vertex.subtract(&tangent.multiply(s))
            ));

            vertex0 = vertex;
            tangent0 = tangent;
        }
        Self { 
            vertices: vertices.to_vec(), 
            controls 
        }
    }

    pub(crate) fn to_poly_line(&self, scale: f64) -> Result<Vec<Point>,CommandError> {
        // I don't just want to put equally spaced points, I want what appears to be called an adaptive bezier:
        // https://agg.sourceforge.net/antigrain.com/research/adaptive_bezier/index.html 
        // I found a Javascript translation of that here: https://github.com/mattdesl/adaptive-bezier-curve, 
        // I also found a rust translation of that javascript translation (https://crates.io/crates/adaptive-bezier).
        // I'm not comfortable with it, since it uses it's own vector structure which pulls in a huge library,
        // but it works, so.... 
        let mut result = Vec::new();
        let mut vertices = self.vertices.iter();
        let mut controls = self.controls.iter();
        if let Some(vertex1) = vertices.next() {
            let mut vertex1 = vertex1;
            result.push(vertex1.clone());
            for vertex2 in vertices {
                if let Some((c1,c2)) = controls.next() {
                    let curve = adaptive_bezier_curve(
                        vertex1.to_vector_2(),
                        c1.to_vector_2(),
                        c2.to_vector_2(),
                        vertex2.to_vector_2(),
                        scale
                    );
                    // convert back to points.
                    for point in curve.iter().take(curve.len() - 2).skip(1) {
                        result.push((point[0], point[1]).try_into()?);
                    }
                }
                result.push(vertex2.clone());
                vertex1 = vertex2;
            }

        };

        Ok(result)

    }

}

pub(crate) fn bezierify_points(line: &[Point], scale: f64) -> Result<Vec<Point>,CommandError> {
    let bezier = PolyBezier::from_poly_line(line);
    bezier.to_poly_line(scale)
}


pub(crate) fn find_curve_making_point(start_point: &Point, end_point: &Point) -> Point {
    // This function creates a phantom point which can be used to give an otherwise straight ending segment a bit of a curve.
    let parallel = start_point.subtract(end_point);
    // I want to switch the direction of the curve in some way that looks random, but is reproducible.
    // The easiest way I can think of is basically to base it off of whether the integral part of a value is even.
    let is_even = start_point.semi_random_toggle();
    let perpendicular = parallel.perpendicular(is_even);
    let normalized = perpendicular.normalized();
    end_point.add(&normalized)
}


pub(crate) fn bezierify_points_with_phantoms(before: Option<&Point>, line: &[Point], after: Option<&Point>, scale: f64) -> Result<Vec<Point>,CommandError> {
    // create the bezier
    let bezier = PolyBezier::from_poly_line_with_phantoms(before,line,after);
    // convert that to a polyline.
    bezier.to_poly_line(scale)
}

#[cfg(test)]
mod test {
    use super::PolyBezier;

    #[test]
    fn test_bezier() {
    
        let pos = vec![
            (0.5, 0.5).try_into().unwrap(),
            (1.0, -0.5).try_into().unwrap(),
            (1.5, 1.0).try_into().unwrap(),
            (2.25, 1.1).try_into().unwrap(),
            (2.6, -0.5).try_into().unwrap(),
            (3.0, 0.5).try_into().unwrap(),
        ];
    
        let curves = PolyBezier::from_poly_line(&pos);
    
        let expected = vec![
            (
                (0.5, 0.5).try_into().unwrap(),
                (0.6666666666666666, 0.16666666666666669).try_into().unwrap(),
                (0.6666666666666667, -0.6666666666666666).try_into().unwrap(),
                (1.0, -0.5).try_into().unwrap(),
            ),
            (
                (1.0, -0.5).try_into().unwrap(),
                (1.4714045207910318, -0.26429773960448416).try_into().unwrap(), 
                (1.1755270999091973, 0.5846746878837725).try_into().unwrap(), 
                (1.5, 1.0).try_into().unwrap(),
            ),
            (
                (1.5, 1.0).try_into().unwrap(),
                (1.655273081384295, 1.1987495441718978).try_into().unwrap(), 
                (2.100850731900237, 1.3033853655905858).try_into().unwrap(), 
                (2.25, 1.1).try_into().unwrap(),
            ),
            (
                (2.25, 1.1).try_into().unwrap(),
                (2.572851825487011, 0.6597475106995304).try_into().unwrap(), 
                (2.1736888549287925, -0.15895108394303398).try_into().unwrap(), 
                (2.6, -0.5).try_into().unwrap(),
            ),
            (
                (2.6, -0.5).try_into().unwrap(),
                (2.8803404821067753, -0.7242723856854201).try_into().unwrap(), 
                (2.8666666666666667, 0.16666666666666669).try_into().unwrap(), 
                (3.0, 0.5).try_into().unwrap(),
            )
        ];
    
        let mut i = 0;
        while let Some(curve) = curves.segment_at(i) {
            let expected_curve = &expected[i];
            assert_eq!(curve.0,&expected_curve.0,"At curve {i}, point 0");
            assert_eq!(curve.1,&expected_curve.1,"At curve {i}, point 1");
            assert_eq!(curve.2,&expected_curve.2,"At curve {i}, point 2");
            assert_eq!(curve.3,&expected_curve.3,"At curve {i}, point 3");
            i += 1;
        }
    
    /* python output:
    [
    [
    (0.500000000000000, 0.500000000000000), 
    (0.666666666666667, 0.166666666666667), 
    (0.666666666666667, -0.666666666666667), 
    (1.00000000000000, -0.500000000000000)
    ], [
    (1.47140452079103, -0.264297739604484), 
    (1.17552709990920, 0.584674687883773), 
    (1.50000000000000, 1.00000000000000)
    ], [
    (1.65527308138430, 1.19874954417190), 
    (2.10085073190024, 1.30338536559059), 
    (2.25000000000000, 1.10000000000000)
    ], [
    (2.57285182548701, 0.659747510699530), 
    (2.17368885492879, -0.158951083943034), 
    (2.60000000000000, -0.500000000000000)
    ], [
    (2.88034048210678, -0.724272385685420), 
    (2.86666666666667, 0.166666666666667), 
    (3.00000000000000, 0.500000000000000)
    ]
    ]
    */
    
    }
    
    
}
