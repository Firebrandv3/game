use coord::prelude::*;
use std::f32::consts::SQRT_2;
use std::f32::INFINITY;
use std::cmp::Ordering;
use std::cmp::Ord;

#[derive(PartialEq, Debug)]
pub struct Cuboid {
    middle: Vec3<f32>,
    radius: Vec3<f32>,
}

#[derive(PartialEq, Debug)]
pub struct Resolution {
    pub center: Vec3<f32>,
    pub correction: Vec3<f32>,
}

#[derive(PartialEq, Debug)]
pub enum ResolutionTti {
    WillColide{ tti: f32, normal: Vec3<f32> }, // tti can be 0.0 when they will overlap in future
    Touching{ normal: Vec3<f32> }, // happens if direction is inanother than Primitive
    Overlapping{ since: f32 },
}

#[derive(PartialEq, Debug)]
pub enum Primitive {
    Cuboid { cuboid: Cuboid },
    //add more here
}

pub trait Collider<'a> {
    type Iter: Iterator<Item = Primitive>;

    fn get_nearby(&'a self, col: &Primitive) -> Self::Iter;
}

pub const PLANCK_LENGTH : f32 = 0.001; // smallest unit of meassurement in collision, no guarantees behind this point

/*
  Resolution is done the following way: we evaluate if to Primitives overlap.
  When they everlap we calculate the center of mass inside the overlapping area (currently center of mass = center)
  We then calcululate a vector beginning from the center of mass ob the overlapping area. to the border of the overlapping area.
  The directin of the fector should be directly towards the center of mass of the second Primitive.
*/

impl Resolution {
    pub fn is_touch(&self) -> bool {self.correction.length() < PLANCK_LENGTH}
}

impl Primitive {
    // CollisionResolution is the minimal movement of b to avoid overlap, but allow touch with self
    pub fn resolve_col(&self, b: &Primitive) -> Option<Resolution> {
        match self {
            Primitive::Cuboid { cuboid: a } => {
                match b {
                    Primitive::Cuboid { cuboid: b } => {
                        a.cuboid_col(b)
                    },
                }
            },
        }
    }

    // Time to impact of b with self when b travels in dir
    // 1. paramter: Time, defined as multiples of dir which can be applied before touching
    //    - positive i am on my way to a Collision
    //    - 0: right now colliding
    //    - negative: i am colliding and overlapping by this much
    //    - Infinite: i will never collide
    // 2. parameter: Normal facing away from Primitive at position of impact
    pub fn time_to_impact(&self, b: &Primitive, dir: &Vec3<f32>) -> Option<ResolutionTti> {
        match self {
            Primitive::Cuboid { cuboid: a } => {
                match b {
                    Primitive::Cuboid { cuboid: b } => {
                        a.cuboid_tti(b, dir)
                    },
                }
            },
        }
    }

    pub fn move_by(&mut self, delta: &Vec3<f32>) {
        match self {
            Primitive::Cuboid { cuboid: a } => a.middle += *delta,
        }
    }

    pub fn center_of_mass(&self) -> Vec3<f32> {
        match self {
            Primitive::Cuboid { cuboid: a } => a.middle,
        }
    }

    // when using the collision center, the outer_aproximation_sphere can be minimal
    // implement it fast!
    pub fn col_center(&self) -> Vec3<f32> {
        match self {
            Primitive::Cuboid { cuboid: a } => a.middle,
        }
    }

    // returns the 3 radii of a spheroid where the object fits exactly in
    // implement it fast!
    //TODO: evaluate if this is a so fast method for checking somewhere actually
    pub fn col_aprox_rad(&self) -> Vec3<f32> {
        match self {
            Primitive::Cuboid { cuboid: a } => a.radius * SQRT_2, // SQRT(2) is correct for sphere, havent it checked for an spheroid tbh
        }
    }

    // returns a cube where the object fits in exactly
    // implement it fast!
    pub fn col_aprox_abc(&self) -> Vec3<f32> {
        match self {
            Primitive::Cuboid { cuboid: a } => a.radius,
        }
    }
}

impl Primitive {
    pub fn new_cuboid(middle: Vec3<f32>, radius: Vec3<f32>) -> Self {
        Primitive::Cuboid{ cuboid: Cuboid::new(middle, radius) }
    }
}

impl Cuboid {
    pub fn new(middle: Vec3<f32>, radius: Vec3<f32>) -> Self {
        Cuboid {
            middle,
            radius,
        }
    }

    fn vector_touch_border(radius: Vec3<f32>, direction: Vec3<f32>) -> Vec3<f32> {
        let first_hit = radius / direction;
        let first_hit = first_hit.map(|e| e.abs());
        let min = if first_hit.x <= first_hit.y && first_hit.x <= first_hit.z {
            first_hit.x
        } else  if first_hit.y <= first_hit.x && first_hit.y <= first_hit.z {
            first_hit.y
        } else {
            first_hit.z
        };
        return direction * min;
    }

    fn cuboid_col(&self, b: &Cuboid) -> Option<Resolution> {
        let a = self;
        let la = a.lower();
        let ua = a.upper();
        let lb = b.lower();
        let ub = b.upper();
        if ua.x >= lb.x && la.x <= ub.x &&
           ua.y >= lb.y && la.y <= ub.y &&
           ua.z >= lb.z && la.z <= ub.z {
                  //collide or touch
                  let col_middle = (*a.middle() + *b.middle()) / 2.0;
                  let col_radius = *a.middle() - *b.middle();
                  let col_radius = vec3!(col_radius.x.abs(), col_radius.y.abs(), col_radius.z.abs());
                  let col_radius = col_radius - *a.radius() - *b.radius();

                  let mut direction = *b.middle() - col_middle;
                  if direction == vec3!(0.0, 0.0, 0.0) {
                      direction = vec3!(0.0, 0.0, 1.0);
                  }
                  let force = Cuboid::vector_touch_border(col_radius, direction);
                  let force = force.map(|e| if e.abs() < PLANCK_LENGTH {0.0} else {e}); // apply PLANCK_LENGTH to force
                  return Some(Resolution{
                      center: col_middle,
                      correction: force,
                  });
                  /*


                  let moved = *b.middle() - *a.middle();
                  let abs_moved = vec3!(moved.x.abs(), moved.y.abs(), moved.z.abs());
                  let border_diff = *a.radius() - abs_moved;
                  let border_diff_abs = abs_moved - *a.radius() - *b.radius();
                  let border_diff_abs = vec3!(border_diff_abs.x.abs(), border_diff_abs.y.abs(), border_diff_abs.z.abs());
                  //println!("");
                  //println!("moved {}      abs {}       border_diff {}           border_diff_abs {}", moved, abs_moved, border_diff, border_diff_abs);
                  let signed_diff_to_border;
                  let signed_relevant_b_radius;

                  // test which is nearest
                  let nearest_fak = if border_diff_abs.x <= border_diff_abs.y && border_diff_abs.x <= border_diff_abs.z {
                      vec3!(if b.middle().x < a.middle().x {-1.0} else {1.0}, 0.0, 0.0)
                  } else if border_diff_abs.y <= border_diff_abs.x && border_diff_abs.y <= border_diff_abs.z {
                      vec3!(0.0, if b.middle().y < a.middle().y {-1.0} else {1.0}, 0.0)
                  } else {
                      if !(border_diff_abs.z <= border_diff_abs.x && border_diff_abs.z <= border_diff_abs.y) {
                           //println!("border_diff: {}", border_diff);
                           assert!(false);
                      }
                      vec3!(0.0, 0.0, if b.middle().z < a.middle().z {-1.0} else {1.0})
                  };
                  signed_diff_to_border = border_diff * nearest_fak;
                  signed_relevant_b_radius = *b.radius() * nearest_fak;

                  let point = *b.middle() + signed_diff_to_border;
                  let correction = signed_diff_to_border + signed_relevant_b_radius;

                  //println!("point {}, correction {}, signed_diff_to_border {}, relevant_a_radius {}", point, correction, signed_diff_to_border, signed_relevant_b_radius);

                  return Some(Resolution{
                      point,
                      correction,
                  });*/
            };
        None
    }

    fn cuboid_tti(&self, b: &Cuboid, dir: &Vec3<f32>) -> Option<ResolutionTti> {
        //calculate areas which collide based on dir
        // e.g. area.x is the x cordinate of the area
        let a = self;
        let a_middle_elem = a.middle.elements();
        let b_middle_elem = b.middle.elements();
        let a_radius_elem = a.radius.elements();
        let b_radius_elem = b.radius.elements();
        let mut a_area = [0.0; 3];
        let mut b_area = [0.0; 3];
        let mut normals: [Vec3<f32>; 3] = [vec3!(0.0, 0.0, 0.0); 3];
        let mut tti_raw: [f32; 3] = [0.0; 3];
        let mut tti: [f32; 3] = [0.0; 3];
        let mut minimal_collision_tti: [f32; 3] = [0.0; 3]; //minimal tti value which equals a collision is already happening
        let dire = dir.elements();
        //println!("a_middle_elem {:?}; b_middle_elem {:?}", a_middle_elem, b_middle_elem);
        //needs to be calculated for every area of the cuboid, happily it's not rotated, so its just the 3 axis
        for i in 0..3 {
            if dire[i] == 0.0 {
                //area is not filled correctly in this case, we compare middle
                let midr = (a_middle_elem[i] - b_middle_elem[i]).abs();
                let perimeterr = (a_radius_elem[i] + b_radius_elem[i]);
                minimal_collision_tti[i] = -INFINITY;
                //println!("midr {:?}; perimeterr {:?}", midr, perimeterr);
                tti_raw[i] = if midr + PLANCK_LENGTH > perimeterr && midr - PLANCK_LENGTH < perimeterr {
                    0.0
                } else {

                    if midr >= perimeterr {
                        INFINITY // no movement and no collsision
                    } else {
                        -INFINITY // as value for there is a collision
                    }
                };
                if tti_raw[i].is_sign_negative() && // it detects collision, detects -INFINITY
                   midr >= (a_radius_elem[i] + b_radius_elem[i]) {// but distance is higher than radius
                       tti[i] = INFINITY; //no collision will ocur, like ever
                   } else {
                       tti[i] = tti_raw[i];
                       if tti[i] > -PLANCK_LENGTH && tti[i] < PLANCK_LENGTH { // PLANCK LENGTH correction
                           tti[i] = 0.0
                       }
                 }
                 if a_middle_elem[i] < b_middle_elem[i] {
                     normals[i] = vec3!(if i == 0 {1.0} else {0.0}, if i == 1 {1.0} else {0.0}, if i == 2 {1.0} else {0.0});
                 } else if a_middle_elem[i] > b_middle_elem[i] {
                     normals[i] = vec3!(if i == 0 {-1.0} else {0.0}, if i == 1 {-1.0} else {0.0}, if i == 2 {-1.0} else {0.0});
                 }
            } else {
                if dire[i] < 0.0 {
                    a_area[i] = a_middle_elem[i] + a_radius_elem[i];
                    b_area[i] = b_middle_elem[i] - b_radius_elem[i];
                    normals[i] = vec3!(if i == 0 {1.0} else {0.0}, if i == 1 {1.0} else {0.0}, if i == 2 {1.0} else {0.0});
                } else if dire[i] > 0.0 {
                    a_area[i] = a_middle_elem[i] - a_radius_elem[i];
                    b_area[i] = b_middle_elem[i] + b_radius_elem[i];
                    normals[i] = vec3!(if i == 0 {-1.0} else {0.0}, if i == 1 {-1.0} else {0.0}, if i == 2 {-1.0} else {0.0});
                } else {
                    panic!("we checked above that dire[i] must not be 0.0");
                }
                //println!("a_area {:?}; b_area {:?}", a_area, b_area);
                minimal_collision_tti[i] = - (a_radius_elem[i] + b_radius_elem[i]) * 2.0 / dire[i].abs();
                tti_raw[i] = (a_area[i] - b_area[i]) / dire[i];
                if tti_raw[i].is_sign_negative() && // it detects collision, detects -INFINITY
                   (a_area[i] - b_area[i]).abs() >= (a_radius_elem[i] + b_radius_elem[i]) * 2.0 {// but distance is higher than radius
                       tti[i] = INFINITY; //no collision will ocur, like ever
                   } else {
                       tti[i] = tti_raw[i];
                       if tti[i] > -PLANCK_LENGTH && tti[i] < PLANCK_LENGTH { // PLANCK LENGTH correction
                           tti[i] = 0.0
                       }
                 }
            }
        }
        // tti now contains a value per coordinate. pos=will colide in, 0=touches right now, negative=is colliding since, INF=will never collide
        // check the number of collisions now. negative number is Collision
        // 0x = corner
        // 1x = edge
        // 2x = area
        // 3x = cuboid

        //println!("tti_raw {:?}", tti_raw);
        println!("tti {:?}", tti);

        // i will check all 3 areas, if after the applying of the movement, others axis will also collid
        // e.g tti (3,4,5) minimum_col (-3,-3,-3)
        //now after 3 ticks, 4 and 5 still dont Collide
        //but after 5 ticks, 3 is -2 and 4 is -1. and this are still collidung because of minimal collide.
        //so this is our collisison here

        if tti[0].is_sign_negative() && tti[1].is_sign_negative() && tti[2].is_sign_negative() {
            if tti[0] >= tti[1] && tti[0] >= tti[2] {
                return  Some(ResolutionTti::Overlapping{ since: -tti[0] });
            }
            if tti[1] >= tti[2] && tti[1] >= tti[0] {
                return  Some(ResolutionTti::Overlapping{ since: -tti[1] });
            }
            if tti[2] >= tti[0] && tti[2] >= tti[1] {
                return  Some(ResolutionTti::Overlapping{ since: -tti[2] });
            }
            return  Some(ResolutionTti::Overlapping{ since: -tti[0] }); // UNREACHABLE, except for some infinity stuff
        }

        #[derive(Debug)]
        struct TtiValueIndex {
            value: f32,
            index: usize,
        }

        impl Ord for TtiValueIndex {
            fn cmp(&self, other: &Self) -> Ordering {
                if self.value.is_infinite() && other.value.is_infinite() {
                    return Ordering::Equal;
                }
                if self.value.is_sign_negative() && other.value.is_sign_negative() {
                    return Ordering::Equal; // we dont want negative
                }
                if self.value.is_sign_negative() {
                    return Ordering::Greater; // be to the end
                }
                if other.value.is_sign_negative() {
                    return Ordering::Less; // be to the end
                }
                if self.value < other.value {
                    return Ordering::Less;
                }
                if self.value > other.value {
                    return Ordering::Greater;
                }
                return Ordering::Equal;
            }
        }

        impl PartialOrd for TtiValueIndex {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }

        impl PartialEq for TtiValueIndex {
            fn eq(&self, other: &Self) -> bool {
                (self.value, &self.index) == (other.value, &other.index)
            }
        }

        impl Eq for TtiValueIndex { }

        // e.g. (-INF, 4, 2), sort for tti  --> (2, 4)
        let mut to_test = [TtiValueIndex{value: tti[0], index: 0}, TtiValueIndex{value: tti[1], index: 1}, TtiValueIndex{value: tti[2], index: 2} ];
        let mut potentialtouch_index : Option<usize> = None;
        let mut potentialtouch_normal : Option<Vec3<f32>> = None;
        let mut potentialcollide_index : Option<usize> = None;
        let mut potentialcollide_normal : Option<Vec3<f32>> = None;
        to_test.sort();
        println!("to_test: {:?}", to_test);
        for i in 0..3 {
            if to_test[i].value >= 0.0 && to_test[i].value.is_finite() {
                //check if others collide after time
                let o1 = (i+1) % 3;
                let o2 = (i+2) % 3;
                let o1_i = to_test[o1].index;
                let o2_i = to_test[o2].index;
                // we only shift the value when it actually moves, otherwise min_col is -INF
                let o1_shifted_value = if dire[o1_i] != 0.0 {to_test[o1].value - to_test[i].value} else {to_test[o1].value};
                let o2_shifted_value = if dire[o2_i] != 0.0 {to_test[o2].value - to_test[i].value} else {to_test[o2].value};
                //println!("i {}", i);
                //println!("yay: {}, o1 {}, o2 {}", to_test[i].value, o1_shifted_value, o2_shifted_value);
                //println!("max: o1 {}, o2 {}", minimal_collision_tti[o1_i], minimal_collision_tti[o2_i]);
                //println!("dire {:?}", dire);
                //println!("shifted {} {}", o1_shifted_value, o2_shifted_value);
                if (o1_shifted_value < 0.0 || o1_shifted_value == 0.0 && dire[o1_i] != 0.0) && (o1_shifted_value > minimal_collision_tti[o1_i] || (minimal_collision_tti[o1_i].is_infinite() /*&& tti[o1_i] != 0.0*/)) &&
                   (o2_shifted_value < 0.0 || o2_shifted_value == 0.0 && dire[o2_i] != 0.0) && (o2_shifted_value > minimal_collision_tti[o2_i] || (minimal_collision_tti[o2_i].is_infinite() /*&& tti[o2_i] != 0.0*/)) {
                       //yep it does, and it's the samllest because to_test was sorted. so output it
                       if dire[to_test[i].index] == 0.0 {
                           // should be return  Some(ResolutionTti::Touching{ normal: normals[to_test[i].index]});
                           if potentialtouch_index.is_none() {
                               potentialtouch_index = Some(i);
                               potentialtouch_normal = Some(normals[to_test[i].index]);
                           }
                       } else {
                           if potentialcollide_index.is_none() {
                               potentialcollide_index = Some(i);
                               potentialcollide_normal = Some(normals[to_test[i].index]);
                           } else if to_test[i].value <= to_test[potentialcollide_index.unwrap()].value {//enge is when 2 or more collect at exact same time
                               if let Some(ref mut nor) = potentialcollide_normal {
                                   *nor += normals[to_test[i].index];
                               }
                           }
                       }
                   }
            }
        }

        if let Some(i) = potentialcollide_index {
            println!("returning index: {}, val {}, nor{}", i,  to_test[i].value, potentialcollide_normal.unwrap());
            return  Some(ResolutionTti::WillColide{ tti: to_test[i].value, normal: potentialcollide_normal.unwrap()});
        }

        if let Some(i) = potentialtouch_index {
            return  Some(ResolutionTti::Touching{ normal: potentialtouch_normal.unwrap()});
        }


        /*
        let mut to_test_index = 0;
        if tti[0] >= 0.0 && (tti[0] <= tti[1] || tti[1] < 0.0) &&  (tti[0] <= tti[2] || tti[2] < 0.0) {
            to_test[to_test_index] = 0;
            to_test_index += 1;
        }
        if tti[1] >= 0.0 && (tti[1] <= tti[0] || tti[0] < 0.0) &&  (tti[1] <= tti[2] || tti[2] < 0.0) {
            to_test[to_test_index] = 1;
            to_test_index += 1;
        }
        if tti[2] >= 0.0 && (tti[2] <= tti[0] || tti[0] < 0.0) &&  (tti[2] <= tti[1] || tti[1] < 0.0) {
            to_test[to_test_index] = 2;
            to_test_index += 1;
        }*/



        /*
        for i in 0..3 {
            let t = tti[i];
            if t.is_sign_positive() {
                if t.is_infinite() {

                } else {
                    let mut will_collide = true;
                    let mut will_touch = true;
                    for j in 0..3 {
                        //apply movement to other
                        if j != i && !(tti[j] - t).is_sign_negative() {
                            will_collide = false;
                        }
                        if j != i && (tti[j] - t) != 0.0 {
                            will_touch = false;
                        }
                    }
                    if will_collide {
                        println!("          TTI_RESULT: {:?}", (t, normals[i]));
                        return  Some(ResolutionTti::WillColide{ tti: t, normal: normals[i]});
                    }
                    if will_touch && t == 0.0 && dire[i] != 0.0 {
                        println!("          TTI_RESULT: {:?}", (t, normals[i]));
                        return  Some(ResolutionTti::WillColide{ tti: t, normal: normals[i]});
                    }
                }
            } else {
                //sign_negative
                if t.is_infinite() {

                } else {
                    let mut is_already_colliding = true;
                    for j in 0..3 {
                        //check if others also negative
                        if j != i && tti[j].is_sign_positive() {
                            is_already_colliding = false;
                            break;
                        }
                    }
                    if is_already_colliding {
                        println!("          TTI_RESULT: {:?}", (t, normals[i]));
                        return  Some(ResolutionTti::Overlapping{ since: -t });
                    }
                }
            }
        }*/


        return None;
    }

    #[allow(dead_code)] pub fn lower(&self) -> Vec3<f32> {
        self.middle - self.radius
    }

    #[allow(dead_code)] pub fn upper(&self) -> Vec3<f32> {
        self.middle + self.radius
    }

    #[allow(dead_code)] pub fn middle(&self) -> &Vec3<f32> { &self.middle }
    #[allow(dead_code)] pub fn middle_mut(&mut self) -> &mut Vec3<f32> { &mut self.middle }
    #[allow(dead_code)] pub fn radius(&self) -> &Vec3<f32> { &self.radius }
    #[allow(dead_code)] pub fn radius_mut(&mut self) -> &mut Vec3<f32> { &mut self.radius }
}
