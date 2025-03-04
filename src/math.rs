use std::f32::consts::PI;

use crate::shape::{Curve, Line, Quad};
use crate::vector::Vector2;

// TODO is this needed?
//#[derive(Debug, Clone, Copy)]
//pub struct SignedDistance(pub f32);

// TODO check if needed
//impl PartialOrd for ContourSignedDistance {
//    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
//        self.distance.partial_cmp(&other.distance)
//    }
//}

/// Distance from pixel to contour
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Distance {
    pub extended_dist: f32,
    pub real_dist: f32,
    pub orthogonality: f32,
    pub sign: f32,
}

impl Distance {
    pub const MAX: Self = Distance {
        extended_dist: f32::MAX,
        real_dist: f32::MAX,
        orthogonality: 0.0,
        sign: f32::NAN,
    };

    #[inline]
    pub fn real_signed(self) -> f32 {
        // Maybe use a struct for sign to ensure it's not a zero.
        self.sign * self.real_dist
    }

    #[inline]
    pub fn pseudo_signed(self) -> f32 {
        // Maybe use a struct for sign to ensure it's not a zero.
        self.sign * self.extended_dist
    }
}

impl PartialOrd for Distance {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        use std::cmp::Ordering;
        let diff = self.real_dist - other.real_dist;
        match diff.abs().partial_cmp(&0.01) {
            Some(Ordering::Less) => {
                other.orthogonality.partial_cmp(&self.orthogonality)
            }
            Some(Ordering::Greater) => {
                self.real_dist.partial_cmp(&other.real_dist)
            }
            Some(Ordering::Equal) => {
                other.orthogonality.partial_cmp(&self.orthogonality)
            }
            None => None,
        }
    }
}

pub fn line_signed_distance(line: &Line, point: Vector2<f32>) -> Distance {
    let p0 = line.from;
    let p1 = line.to;
    let p = point;

    let p_p0 = p - p0;
    let p1_p0 = p1 - p0; // Is also the direction

    // Find the "t" from the line function
    // and restrict it to an interval [0.0, 1.0].
    let extended_pos = p_p0.dot(p1_p0) / p1_p0.dot(p1_p0);
    let real_pos = extended_pos.clamp(0.0, 1.0);

    // Put "t" in bezier function and get the closest
    // point to the current pixel "p"
    let extended_bezier = p0 + extended_pos * p1_p0;
    let extend_bezier_p = extended_bezier - p;
    let bezier = p0 + real_pos * p1_p0;
    let bezier_p = bezier - p;

    // Get the distance from current pixel "p" to bezier line.
    let real_dist = bezier_p.magnitude();
    let extended_dist = extend_bezier_p.magnitude();

    // Invert the vector to get distance from bezier line to "p".
    let p_bezier = std::ops::Neg::neg(bezier_p);
    let ortho: f32 = if p_bezier.is_zero() {
        0.0
    } else {
        p1_p0.normalize().cross(p_bezier.normalize())
    };
    let sign = ortho.signum();
    let orthogonality = ortho.abs();

    Distance {
        extended_dist,
        real_dist,
        orthogonality,
        sign,
    }
}

pub fn quad_signed_distance(quad: &Quad, point: Vector2<f32>) -> Distance {
    let p0 = quad.from;
    let p1 = quad.ctrl;
    let p2 = quad.to;
    let p = point;

    let v = p - p0;
    let v1 = p1 - p0;
    let v2 = p2 - 2.0 * p1 + p0;
    // quadratic Bezier curve:
    // (v2 · v2)t^3 + 3(v1 · v2)t^2 + (2*v1 · v1 − v2 · v)t − v1 · v = 0
    // general quadratic:
    // a * t^3 + b * t^2 + c * t + d = 0

    let a = v2.dot(v2);
    let b = 3.0 * v1.dot(v2);
    let c = 2.0 * v1.dot(v1) - v2.dot(v);
    let d = -v1.dot(v);

    // Get roots:
    let roots = cubic_roots(a, b, c, d);

    let mut extended_pos = 0.0;
    let mut real_pos = 0.0;
    let mut closest_bezier = Vector2::new(f32::MAX, f32::MAX);
    let mut smallest_dist2 = f32::MAX; // Not square rooted

    // Compare all roots to find the closest "t" and smallest distance.
    for r in roots.iter().flatten() {
        // <-- automatically filters out Options with None
        // Use clamped root in the quadratic function.
        let t = r.clamp(0.0, 1.0);
        let bezier = t * t * v2 + 2.0 * t * v1 + p0;

        // Then compare the distances for each root.
        let dist2 = (bezier - p).magnitude2();
        if dist2 < smallest_dist2 {
            extended_pos = *r;
            real_pos = t;
            closest_bezier = bezier;
            smallest_dist2 = dist2;
        }
    }

    // Get the distance from current pixel "p" to bezier line.
    let extended_bezier =
        extended_pos * extended_pos * v2 + 2.0 * extended_pos * v1 + p0;
    let extended_dist = (extended_bezier - p).magnitude();
    let real_dist = smallest_dist2.sqrt();

    // Invert the vector to get distance from bezier line to "p". TODO explain
    let dir = 2.0 * v2 * real_pos + 2.0 * v1;
    let p_bezier = p - closest_bezier;
    let ortho: f32 = if p_bezier.is_zero() || dir.is_zero() {
        0.0
    } else {
        dir.normalize().cross(p_bezier.normalize())
    };
    let sign = ortho.signum();
    let orthogonality = ortho.abs();

    Distance {
        extended_dist,
        real_dist,
        orthogonality,
        sign,
    }
}

#[allow(unused)]
pub fn curve_signed_distance(curve: &Curve, point: Vector2<f32>) -> Distance {
    unimplemented!()
}

fn quadratic_roots(a: f32, b: f32, c: f32) -> [Option<f32>; 2] {
    let discriminant = b * b - 4.0 * a * c;

    if a == 0.0 {
        if b == 0.0 {
            return [None, None];
        }
        return [Some(-c / b), None];
    }

    if discriminant < 0.0 {
        [None, None]
    } else if discriminant > 0.0 {
        let discriminant_sqrt = discriminant.sqrt();
        let a2 = 1.0 / (2.0 * a);
        // Root 1
        let x1 = -(b + discriminant_sqrt) * a2;
        // Root 2
        let x2 = (discriminant_sqrt - b) * a2;

        [Some(x1), Some(x2)]
    } else {
        let extreme_x = -0.5 * b / a;
        [Some(extreme_x), None]
    }
}

fn cubic_roots(a: f32, b: f32, c: f32, d: f32) -> [Option<f32>; 3] {
    if a == 0.0 {
        let roots = quadratic_roots(b, c, d);
        return [roots[0], roots[1], None];
    }

    // All formulas and procedures are explained at: https://mathworld.wolfram.com/CubicFormula.html

    let mut b = b / a;
    let c = c / a;
    let d = d / a;

    let q = (b * b - 3.0 * c) / 9.0; // TODO explain why we negate numerator
    let r = (2.0 * b * b * b + 27.0 * d - 9.0 * c * b) / 54.0;

    let qqq = q * q * q;
    let rr = r * r;
    let third = 1.0 / 3.0;
    b *= third;

    if rr > qqq {
        // D > 0.0
        // Then there is only one root.
        let s = -r.signum() * (r.abs() + (rr - qqq).sqrt()).cbrt();
        let x1 = (s + q / s) - b; // TODO exclain // ALSO CAN BE q/s=t WHYY??

        [Some(x1), None, None]
    } else {
        // D <= 0.0, q < 0.0
        // root1 = (2 * sqrt(-q)) * cos(theta/3) - (third * b);
        // root2 = (2 * sqrt(-q)) * cos((theta + 2*pi)/3) - (third * b);
        // root3 = (2 * sqrt(-q)) * cos((theta + 4*pi)/3) - (third * b);
        // root = m * cos((theta + ...)/3) - n;
        let q_sqrt = q.sqrt();
        let two_pi = 2.0 * PI;
        let theta = (r / q_sqrt.powi(3)).acos();
        let m = -2.0 * q_sqrt;
        let x1 = m * (theta * third).cos() - b;
        let x2 = m * ((theta + two_pi) * third).cos() - b;
        let x3 = m * ((theta - two_pi) * third).cos() - b;

        [Some(x1), Some(x2), Some(x3)]
    }
}

/// Takes four points as input where **p0-p1** create first line and **p2-p3** create
/// second line. If they intersect the function returns `true` and if they don't
/// `false` is returned.
///
/// Even tho lines are touching when they are coincident or are on the same line touching,
/// this function will not record it as an intersection.
///
/// Procedure explained at:
/// https://web.archive.org/web/20121001232059/http://paulbourke.net/geometry/lineline2d/
pub fn line_line_intersection(
    line1: &Line,
    line2: &Line,
) -> Option<Vector2<f32>> {
    let p0 = line1.from;
    let p1 = line1.to;
    let p2 = line2.from;
    let p3 = line2.to;

    // TODO adapt this function with in mind that intersections will rarely happen
    let x0_x2 = p0.x - p2.x;
    let y0_y2 = p0.y - p2.y;
    let x1_x0 = p1.x - p0.x;
    let y1_y0 = p1.y - p0.y;
    let x3_x2 = p3.x - p2.x;
    let y3_y2 = p3.y - p2.y;

    let numerator1 = (x3_x2 * y0_y2) - (y3_y2 * x0_x2);
    let numerator2 = (x1_x0 * y0_y2) - (y1_y0 * x0_x2);
    let denominator = (y3_y2 * x1_x0) - (x3_x2 * y1_y0);

    // Even tho lines are touching when they are coincident or are on the
    // same line, this function will not record it as an intersection.
    //if denominator == 0.0 && numerator1 == 0.0 && numerator2 == 0.0 {
    //    // println!("COINCIDENT");
    //    return None;
    //}
    if denominator != 0.0 {
        // Variable of the first line function
        let t1 = numerator1 / denominator;

        // Variable of the second line function
        let t2 = numerator2 / denominator;

        // println!("t1: {}, t2: {}", t1, t2);

        if (0.0..=1.0).contains(&t1) && (0.0..=1.0).contains(&t2) {
            //assert_eq!(line_fn(p0, p1, t1), line_fn(p2, p3, t2));
            return Some(line_fn(p0, p1, t1));
        }
    }

    None
}

// TODO exclain and improve and simplify
/// Sources:
/// https://www.particleincell.com/2013/cubic-line-intersection/
/// https://stackoverflow.com/questions/50129580/program-to-find-line-segment-and-bezier-curve-intersection
pub fn quad_line_intersection(
    quad: &Quad,
    line: &Line,
) -> [Option<Vector2<f32>>; 2] {
    let q0 = quad.from;
    let q1 = quad.ctrl;
    let q2 = quad.to;
    let l0 = line.from;
    let l1 = line.to;

    let mut intersections = [None; 2];
    let a = (q0 + q2 - 2.0 * q1).cross(l1 - l0);
    let b = 2.0 * (q1 - q0).cross(l1 - l0);
    let c = (q0 - l0).cross(l1 - l0);

    let quad_roots = quadratic_roots(a, b, c);

    for (i, root) in quad_roots.iter().flatten().enumerate() {
        if *root >= 0.0 && *root <= 1.0 {
            let quad_p = quadratic_fn(q0, q1, q2, *root);
            // TODO find out what "line_p" represents
            let line_p = (quad_p - l0) / (l1 - l0);
            if (line_p.x >= 0.0 && line_p.x <= 1.0)
                && (line_p.y >= 0.0 && line_p.y <= 1.0)
            {
                // assert_eq!(line_fn(l0, l1, line_p.x), quad_p);
                intersections[i] = Some(line_fn(l0, l1, line_p.x));
            }
            //println!("line: {:?}", line_p);
        }
    }

    //println!("roots: {:?}", quad_roots);

    intersections
}

#[allow(unused)]
pub fn quad_quad_intersection(
    quad1: &Quad,
    quad2: &Quad,
) -> [Option<Vector2<f32>>; 4] {
    // TODO this
    [None, None, None, None]
}

#[allow(unused)]
pub fn curve_line_intersection(
    curve: &Curve,
    line: &Line,
) -> [Option<Vector2<f32>>; 3] {
    todo!()
}

#[allow(unused)]
pub fn curve_quad_intersection(
    curve: &Curve,
    quad: &Quad,
) -> [Option<Vector2<f32>>; 6] {
    todo!()
}

#[allow(unused)]
pub fn curve_curve_intersection(
    curve1: &Curve,
    curve2: &Curve,
) -> [Option<Vector2<f32>>; 9] {
    todo!()
}

/// A line function.
/// - `p0` - line starting point
/// - `p1` - line ending point
/// - `t` - function parameter
// TODO maybe convert to macro!
#[inline]
#[allow(unused)]
pub fn line_fn(p0: Vector2<f32>, p1: Vector2<f32>, t: f32) -> Vector2<f32> {
    p0 + t * (p1 - p0)
}

/// A line function.
/// - `p0` - curve starting point
/// - `p1` - curve control point
/// - `p2` - curve ending point
/// - `t` - function parameter
// TODO maybe convert to macro!
#[inline]
#[allow(unused)]
pub fn quadratic_fn(
    p0: Vector2<f32>,
    p1: Vector2<f32>,
    p2: Vector2<f32>,
    t: f32,
) -> Vector2<f32> {
    p0 + 2.0 * t * (p1 - p0) + t * t * (p2 - 2.0 * p1 + p0)
}

#[test]
fn line_quad_intersection_test() {
    // TODO maybe add more tests
    // TEST 1
    let line = Line {
        from: Vector2::new(4.0, 5.0),
        to: Vector2::new(10.0, 9.0),
    };

    let quad = Quad {
        from: Vector2::new(2.5, 2.0),
        ctrl: Vector2::new(3.0, 8.0),
        to: Vector2::new(10.0, 12.0),
    };

    assert_eq!(quad_line_intersection(&quad, &line), [None, None]);

    // TEST 2
    let line = Line {
        from: Vector2::new(1.0, 1.0),
        to: Vector2::new(10.0, 10.0),
    };

    let quad = Quad {
        from: Vector2::new(2.5, 2.0),
        ctrl: Vector2::new(3.0, 8.0),
        to: Vector2::new(10.0, 12.0),
    };

    assert!(quad_line_intersection(&quad, &line)[0].is_some());
    assert!(quad_line_intersection(&quad, &line)[1].is_none());

    // TEST 3
    let line = Line {
        from: Vector2::new(1.0, 1.0),
        to: Vector2::new(10.0, 10.0),
    };

    let quad = Quad {
        from: Vector2::new(2.0, 2.1),
        ctrl: Vector2::new(3.0, 8.0),
        to: Vector2::new(9.0, 8.0),
    };

    assert!(quad_line_intersection(&quad, &line)[0].is_none());
    assert!(quad_line_intersection(&quad, &line)[1].is_some());
}

#[test]
fn line_intersection_test() {
    // Parallel lines:
    let line1 = Line {
        from: Vector2::new(1.0, 1.0),
        to: Vector2::new(10.0, 1.0),
    };

    let line2 = Line {
        from: Vector2::new(1.0, 20.0),
        to: Vector2::new(10.0, 20.0),
    };

    assert!(line_line_intersection(&line1, &line2).is_none());

    // Coincident lines:
    let line1 = Line {
        from: Vector2::new(1.0, 1.0),
        to: Vector2::new(10.0, 3.0),
    };

    let line2 = Line {
        from: Vector2::new(1.0, 1.0),
        to: Vector2::new(10.0, 3.0),
    };

    assert!(line_line_intersection(&line1, &line2).is_none());

    // Crossing lines:
    let line1 = Line {
        from: Vector2::new(1.0, 1.0),
        to: Vector2::new(10.0, 10.0),
    };

    let line2 = Line {
        from: Vector2::new(1.0, 10.0),
        to: Vector2::new(3.0, 0.0),
    };

    assert!(line_line_intersection(&line1, &line2).is_some());

    // Lines meet at ending point:
    let line1 = Line {
        from: Vector2::new(1.0, 1.0),
        to: Vector2::new(10.0, 10.0),
    };

    let line2 = Line {
        from: Vector2::new(10.0, 10.0),
        to: Vector2::new(20.0, 1.0),
    };

    assert!(line_line_intersection(&line1, &line2).is_some());

    // Not parallel, not crossing lines:
    let line1 = Line {
        from: Vector2::new(1.0, 10.0),
        to: Vector2::new(3.0, 1.0),
    };

    let line2 = Line {
        from: Vector2::new(3.0, 10.0),
        to: Vector2::new(10.0, 15.0),
    };

    assert!(line_line_intersection(&line1, &line2).is_none());

    // On the same line crossing:
    let line1 = Line {
        from: Vector2::new(1.0, 1.0),
        to: Vector2::new(10.0, 10.0),
    };

    let line2 = Line {
        from: Vector2::new(8.0, 8.0),
        to: Vector2::new(20.0, 20.0),
    };

    assert!(line_line_intersection(&line1, &line2).is_none());

    // On the same line but not crossing:
    let line1 = Line {
        from: Vector2::new(1.0, 1.0),
        to: Vector2::new(100.0, 100.0),
    };

    let line2 = Line {
        from: Vector2::new(110.0, 110.0),
        to: Vector2::new(200.0, 200.0),
    };

    assert!(line_line_intersection(&line1, &line2).is_none());
}

#[test]
fn cubic_root_test() {
    let a = 1.0;
    let b = 100.4;
    let c = -100.4;
    let d = -0.29;
    let (_, discriminant, q, r) = test_find_cubic_roots(a, b, c, d);

    assert!(q < 0.0);
    assert!(r < 0.0);
    assert!(discriminant < 0.0);

    let a = 1.0;
    let b = -1.0;
    let c = -1.6;
    let d = 2.5;
    let (_, discriminant, _q, _r) = test_find_cubic_roots(a, b, c, d);

    assert!(discriminant > 0.0);
}

#[allow(unused)]
fn test_find_cubic_roots(
    _a: f32,
    _b: f32,
    _c: f32,
    _d: f32,
) -> (Vec<f32>, f32, f32, f32) {
    let b = _b / _a;
    let c = _c / _a;
    let d = _d / _a;

    let q = (3.0 * c - b * b) / 9.0; // TODO explain why we negate numerator
    let r = (9.0 * b * c - 27.0 * d - 2.0 * b * b * b) / 54.0;
    let qqq = q * q * q;
    let discriminant = qqq + r * r;
    let third = 1.0 / 3.0;

    if discriminant > 0.0 {
        // Then there is only one root.
        let s = (r + discriminant.sqrt()).cbrt();
        let t = (r - discriminant.sqrt()).cbrt();
        if s.is_nan() {
            println!("s: {}", s);
        }
        if t.is_nan() {
            println!("t: {}", t);
        }
        /*let temp = ((discriminant).sqrt() + r.abs()).powf(third);
        let sign = r.signum();
        let r = -sign * (temp + q / temp) - third * b;*/
        let x1 = (s + t) - third * b;

        return (vec![x1], discriminant, q, r);
    }
    let two_pi = 2.0 * PI;
    let theta = (r / (-qqq).sqrt()).acos();
    let m = 2.0 * (-q).sqrt();
    let n = b * third;
    let x1 = m * (theta / 3.0).cos() - n;
    let x2 = m * ((theta + two_pi) / 3.0).cos() - n;
    let x3 = m * ((theta + 2.0 * two_pi) / 3.0).cos() - n;
    if x1.is_nan() {
        println!("x1 je nan D <= 0.0")
    }
    if x2.is_nan() {
        println!("x2 je nan D <= 0.0")
    }
    if x3.is_nan() {
        println!("x3 je nan D <= 0.0")
    }

    (vec![x1, x2, x3], discriminant, q, r)
}
