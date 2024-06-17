#![allow(non_snake_case)]

// Example functions
use savvy::savvy;
use savvy::{IntegerSexp, OwnedIntegerSexp, RealSexp, OwnedRealSexp, ListSexp, OwnedListSexp, TypedSexp};
// use savvy::NotAvailableValue;
// use savvy::r_println;
use clarabel::algebra::*;
use clarabel::solver::*;
use lazy_static::lazy_static;
use regex::Regex;


// // Create an OwnedRealSexp out of a Rust double vector
// fn make_owned_real_sexp(v: &Vec<f64>) -> savvy::Result<savvy::Sexp> {
//     let mut out = OwnedRealSexp::new(v.len())?;
//     for (i, &v) in v.iter().enumerate() {
//         out[i] = v;
//     }
//     out.into()
// }

// impl From<clarabel::solver::SolverStatus> for i32 {
//     fn from(status: SolverStatus) -> Self {
//         status as i32
//     }
// }

// Solve using Clarabel Rust solver.
/// @export
#[savvy]
fn clarabel_solve(m: i32, n: i32, Ai: IntegerSexp, Ap: IntegerSexp, Ax: RealSexp, b: RealSexp, q: RealSexp, Pi: IntegerSexp, Pp: IntegerSexp, Px: RealSexp, cone_spec: ListSexp, r_settings: ListSexp) -> savvy::Result<savvy::Sexp> {
    // QP Example
    let P = if Px.len() > 0 {
	CscMatrix::new(
            n as usize,             // m
            n as usize,             // n
            Pp.as_slice().iter().map(|&x| x as usize).collect(),          // colptr
            Pi.as_slice().iter().map(|&x| x as usize).collect(),          // rowval
            Px.as_slice().to_vec(),          // nzval
	)
    } else {
	CscMatrix::zeros((n as usize, n as usize)) // For P = 0
    };

    // println!("P: {:?}", P);
    
    // assert!(P.check_format().is_ok());    

    let A = {
	CscMatrix::new(
            m as usize,             // m
            n as usize,             // n
            Ap.as_slice().iter().map(|&x| x as usize).collect(),          // colptr
            Ai.as_slice().iter().map(|&x| x as usize).collect(),          // rowval
            Ax.as_slice().to_vec(),          // nzval
	)
    };

    // println!("A: {:?}", A);
    // assert!(A.check_format().is_ok());
    
    let b = b.as_slice().to_vec();
    let q = q.as_slice().to_vec();
    // println!("b: {:?}", b);
    // println!("q: {:?}", q);    
    
    // Handle cones
    lazy_static! {
	static ref ZC: Regex = Regex::new("^z").unwrap();  // ZeroCone
	static ref NNC: Regex = Regex::new("^l").unwrap(); // NonNegativeCone
	static ref SOC: Regex = Regex::new("^q").unwrap(); // Second Order Cone
	static ref EPC: Regex = Regex::new("^ep").unwrap(); // Exponential Cone
	static ref PC: Regex = Regex::new("^p").unwrap();  // Power Cone
	static ref PSDTC: Regex = Regex::new("^s").unwrap();  // PSD Triangle Cone
    }

    let mut cones: Vec::<SupportedConeT<f64>> = Vec::new();
    for (key, value) in cone_spec.iter() {
	let typed_value = value.into_typed();
	if ZC.is_match(key.as_ref()) {
	    match typed_value {
		TypedSexp::Integer(i) => cones.push(ZeroConeT(i.as_slice()[0] as usize)),
		_ => (),
	    }
	} else if NNC.is_match(key.as_ref()) {
	    match typed_value {
		TypedSexp::Integer(i) => cones.push(NonnegativeConeT(i.as_slice()[0] as usize)),
		_ => (),		
	    }
	} else if SOC.is_match(key.as_ref()) {
	    match typed_value {
		TypedSexp::Integer(i) => cones.push(SecondOrderConeT(i.as_slice()[0] as usize)),
		_ => (),
	    }
	} else if EPC.is_match(key.as_ref()) {
	    match typed_value {
		TypedSexp::Integer(i) => for _i in 0..(i.as_slice()[0] as usize) {
		    cones.push(ExponentialConeT());
		}
		_ => (),
	    }
	} else if PSDTC.is_match(key.as_ref()) {
	    match typed_value {
		TypedSexp::Integer(i) => cones.push(PSDTriangleConeT(i.as_slice()[0] as usize)),
		_ => (),
	    }
	} else if PC.is_match(key.as_ref()) {
	    match typed_value {
		TypedSexp::Real(f) => cones.push(PowerConeT(f.as_slice()[0] as f64)),
		_ => (),
	    }
        } else {
	    let msg = format!("Ignoring unknown cone: {}", key);
	    let _ = savvy::io::r_warn(&msg);
	}
    }

    // println!("cones: {:?}", cones);    
    // Update default settings with specified R settings for use below
    let settings = update_settings(r_settings);
    
    let mut solver = DefaultSolver::new(&P, &q, &A, &b, &cones, settings);
    solver.solve();

    let mut obj_val = OwnedRealSexp::new(1)?;
    obj_val[0] = solver.solution.obj_val;

    let mut status = OwnedIntegerSexp::new(1)?;
    status[0] = solver.solution.status as i32 + 1;  // R's one-based index

    let mut solve_time = OwnedRealSexp::new(1)?;
    solve_time[0] = solver.solution.solve_time;

    let mut iterations = OwnedIntegerSexp::new(1)?;
    iterations[0] = solver.solution.iterations as i32;

    let mut r_prim = OwnedRealSexp::new(1)?;
    r_prim[0] = solver.solution.r_prim;

    let mut r_dual = OwnedRealSexp::new(1)?;
    r_dual[0] = solver.solution.r_dual;

    let mut out = OwnedListSexp::new(9, true)?;
    out.set_name_and_value(0, "x", OwnedRealSexp::try_from_slice(solver.solution.x)?)?;
    out.set_name_and_value(1, "z", OwnedRealSexp::try_from_slice(solver.solution.z)?)?;
    out.set_name_and_value(2, "s", OwnedRealSexp::try_from_slice(solver.solution.s)?)?;
    out.set_name_and_value(3, "obj_val", obj_val)?;
    out.set_name_and_value(4, "status", status)?;
    out.set_name_and_value(5, "solve_time", solve_time)?;
    out.set_name_and_value(6, "iterations", iterations)?;
    out.set_name_and_value(7, "r_prim", r_prim)?;
    out.set_name_and_value(8, "r_dual", r_dual)?;

    out.into()
}


fn update_settings(r_settings: ListSexp) -> DefaultSettings<f64> {
    let mut settings = DefaultSettings::default();
    // Should really be using something like structmap (https://github.com/ex0dus-0x/structmap) but
    // unsuccessful so far, so a directly generated implementation
    for (key, value) in r_settings.iter() {
	let typed_value = value.into_typed();
	match key.as_ref() {
	    "max_iter" => 
		match typed_value {
		    TypedSexp::Integer(i) => settings.max_iter = i.as_slice()[0] as u32,
		    _ => settings.max_iter = settings.max_iter,
		},
	    "time_limit" => 
		match typed_value {
		    TypedSexp::Real(f) => settings.time_limit = f.as_slice()[0],
		    _ => settings.time_limit = settings.time_limit,
		},
	    "verbose" => 
		match typed_value {
		    TypedSexp::Logical(b) => settings.verbose = b.as_slice_raw()[0] != 0,
		    _ => settings.verbose = settings.verbose,
		},
	    "max_step_fraction" => 
		match typed_value {
		    TypedSexp::Real(f) => settings.max_step_fraction = f.as_slice()[0],
		    _ => settings.max_step_fraction = settings.max_step_fraction,
		},
	    "tol_gap_abs" => 
		match typed_value {
		    TypedSexp::Real(f) => settings.tol_gap_abs = f.as_slice()[0],
		    _ => settings.tol_gap_abs = settings.tol_gap_abs,
		},
	    "tol_gap_rel" => 
		match typed_value {
		    TypedSexp::Real(f) => settings.tol_gap_rel = f.as_slice()[0],
		    _ => settings.tol_gap_rel = settings.tol_gap_rel,
		},
	    "tol_feas" => 
		match typed_value {
		    TypedSexp::Real(f) => settings.tol_feas = f.as_slice()[0],
		    _ => settings.tol_feas = settings.tol_feas,
		},
	    "tol_infeas_abs" => 
		match typed_value {
		    TypedSexp::Real(f) => settings.tol_infeas_abs = f.as_slice()[0],
		    _ => settings.tol_infeas_abs = settings.tol_infeas_abs,
		},
	    "tol_infeas_rel" => 
		match typed_value {
		    TypedSexp::Real(f) => settings.tol_infeas_rel = f.as_slice()[0],
		    _ => settings.tol_infeas_rel = settings.tol_infeas_rel,
		},
	    "tol_ktratio" => 
		match typed_value {
		    TypedSexp::Real(f) => settings.tol_ktratio = f.as_slice()[0],
		    _ => settings.tol_ktratio = settings.tol_ktratio,
		},
	    "reduced_tol_gap_abs" => 
		match typed_value {
		    TypedSexp::Real(f) => settings.reduced_tol_gap_abs = f.as_slice()[0],
		    _ => settings.reduced_tol_gap_abs = settings.reduced_tol_gap_abs,
		},
	    "reduced_tol_gap_rel" => 
		match typed_value {
		    TypedSexp::Real(f) => settings.reduced_tol_gap_rel = f.as_slice()[0],
		    _ => settings.reduced_tol_gap_rel = settings.reduced_tol_gap_rel,
		},
	    "reduced_tol_feas" => 
		match typed_value {
		    TypedSexp::Real(f) => settings.reduced_tol_feas = f.as_slice()[0],
		    _ => settings.reduced_tol_feas = settings.reduced_tol_feas,
		},
	    "reduced_tol_infeas_abs" => 
		match typed_value {
		    TypedSexp::Real(f) => settings.reduced_tol_infeas_abs = f.as_slice()[0],
		    _ => settings.reduced_tol_infeas_abs = settings.reduced_tol_infeas_abs,
		},
	    "reduced_tol_infeas_rel" => 
		match typed_value {
		    TypedSexp::Real(f) => settings.reduced_tol_infeas_rel = f.as_slice()[0],
		    _ => settings.reduced_tol_infeas_rel = settings.reduced_tol_infeas_rel,
		},
	    "reduced_tol_ktratio" => 
		match typed_value {
		    TypedSexp::Real(f) => settings.reduced_tol_ktratio = f.as_slice()[0],
		    _ => settings.reduced_tol_ktratio = settings.reduced_tol_ktratio,
		},
	    "equilibrate_enable" => 
		match typed_value {
		    TypedSexp::Logical(b) => settings.equilibrate_enable = b.as_slice_raw()[0] != 0,
		    _ => settings.equilibrate_enable = settings.equilibrate_enable,
		},
	    "equilibrate_max_iter" => 
		match typed_value {
		    TypedSexp::Integer(i) => settings.equilibrate_max_iter = i.as_slice()[0] as u32,
		    _ => settings.equilibrate_max_iter = settings.equilibrate_max_iter,
		},
	    "equilibrate_min_scaling" => 
		match typed_value {
		    TypedSexp::Real(f) => settings.equilibrate_min_scaling = f.as_slice()[0],
		    _ => settings.equilibrate_min_scaling = settings.equilibrate_min_scaling,
		},
	    "equilibrate_max_scaling" => 
		match typed_value {
		    TypedSexp::Real(f) => settings.equilibrate_max_scaling = f.as_slice()[0],
		    _ => settings.equilibrate_max_scaling = settings.equilibrate_max_scaling,
		},
	    "linesearch_backtrack_step" => 
		match typed_value {
		    TypedSexp::Real(f) => settings.linesearch_backtrack_step = f.as_slice()[0],
		    _ => settings.linesearch_backtrack_step = settings.linesearch_backtrack_step,
		},
	    "min_switch_step_length" => 
		match typed_value {
		    TypedSexp::Real(f) => settings.min_switch_step_length = f.as_slice()[0],
		    _ => settings.min_switch_step_length = settings.min_switch_step_length,
		},
	    "min_terminate_step_length" => 
		match typed_value {
		    TypedSexp::Real(f) => settings.min_terminate_step_length = f.as_slice()[0],
		    _ => settings.min_terminate_step_length = settings.min_terminate_step_length,
		},
	    "direct_kkt_solver" => 
		match typed_value {
		    TypedSexp::Logical(b) => settings.direct_kkt_solver = b.as_slice_raw()[0] != 0,
		    _ => settings.direct_kkt_solver = settings.direct_kkt_solver,
		},
	    "direct_solve_method" => 
		match typed_value {
		    TypedSexp::String(s) => if let Some(result) = s.to_vec().get(0) {
			settings.direct_solve_method = result.to_string();
		    },
		    _ => settings.direct_solve_method = settings.direct_solve_method,
		},
	    "static_regularization_enable" => 
		match typed_value {
		    TypedSexp::Logical(b) => settings.static_regularization_enable = b.as_slice_raw()[0] != 0,
		    _ => settings.static_regularization_enable = settings.static_regularization_enable,
		},
	    "static_regularization_constant" => 
		match typed_value {
		    TypedSexp::Real(f) => settings.static_regularization_constant = f.as_slice()[0],
		    _ => settings.static_regularization_constant = settings.static_regularization_constant,
		},
	    "static_regularization_proportional" => 
		match typed_value {
		    TypedSexp::Real(f) => settings.static_regularization_proportional = f.as_slice()[0],
		    _ => settings.static_regularization_proportional = settings.static_regularization_proportional,
		},
	    "dynamic_regularization_enable" => 
		match typed_value {
		    TypedSexp::Logical(b) => settings.dynamic_regularization_enable = b.as_slice_raw()[0] != 0,
		    _ => settings.dynamic_regularization_enable = settings.dynamic_regularization_enable,
		},
	    "dynamic_regularization_eps" => 
		match typed_value {
		    TypedSexp::Real(f) => settings.dynamic_regularization_eps = f.as_slice()[0],
		    _ => settings.dynamic_regularization_eps = settings.dynamic_regularization_eps,
		},
	    "dynamic_regularization_delta" => 
		match typed_value {
		    TypedSexp::Real(f) => settings.dynamic_regularization_delta = f.as_slice()[0],
		    _ => settings.dynamic_regularization_delta = settings.dynamic_regularization_delta,
		},
	    "iterative_refinement_enable" => 
		match typed_value {
		    TypedSexp::Logical(b) => settings.iterative_refinement_enable = b.as_slice_raw()[0] != 0,
		    _ => settings.iterative_refinement_enable = settings.iterative_refinement_enable,
		},
	    "iterative_refinement_reltol" => 
		match typed_value {
		    TypedSexp::Real(f) => settings.iterative_refinement_reltol = f.as_slice()[0],
		    _ => settings.iterative_refinement_reltol = settings.iterative_refinement_reltol,
		},
	    "iterative_refinement_abstol" => 
		match typed_value {
		    TypedSexp::Real(f) => settings.iterative_refinement_abstol = f.as_slice()[0],
		    _ => settings.iterative_refinement_abstol = settings.iterative_refinement_abstol,
		},
	    "iterative_refinement_max_iter" => 
		match typed_value {
		    TypedSexp::Integer(i) => settings.iterative_refinement_max_iter = i.as_slice()[0] as u32,
		    _ => settings.iterative_refinement_max_iter = settings.iterative_refinement_max_iter,
		},
	    "iterative_refinement_stop_ratio" => 
		match typed_value {
		    TypedSexp::Real(f) => settings.iterative_refinement_stop_ratio = f.as_slice()[0],
		    _ => settings.iterative_refinement_stop_ratio = settings.iterative_refinement_stop_ratio,
		},
	    "presolve_enable" => 
		match typed_value {
		    TypedSexp::Logical(b) => settings.presolve_enable = b.as_slice_raw()[0] != 0,
		    _ => settings.presolve_enable = settings.presolve_enable,
		},
	    "chordal_decomposition_enable" => 
		match typed_value {
		    TypedSexp::Logical(b) => settings.chordal_decomposition_enable = b.as_slice_raw()[0] != 0,
		    _ => settings.chordal_decomposition_enable = settings.chordal_decomposition_enable,
		},
	    "chordal_decomposition_merge_method" => 
		match typed_value {
		    TypedSexp::String(s) => if let Some(result) = s.to_vec().get(0) {
			settings.chordal_decomposition_merge_method = result.to_string();
		    },
		    _ => settings.chordal_decomposition_merge_method = settings.chordal_decomposition_merge_method,
		},
	    "chordal_decomposition_compact" => 
		match typed_value {
		    TypedSexp::Logical(b) => settings.chordal_decomposition_compact = b.as_slice_raw()[0] != 0,
		    _ => settings.chordal_decomposition_compact = settings.chordal_decomposition_compact,
		},
	    "chordal_decomposition_complete_dual" => 
		match typed_value {
		    TypedSexp::Logical(b) => settings.chordal_decomposition_complete_dual = b.as_slice_raw()[0] != 0,
		    _ => settings.chordal_decomposition_complete_dual = settings.chordal_decomposition_complete_dual,
		},
	    _ => (),
	}
    }
    
    settings
}
// if let Some(result) = s.to_vec().get(0) {
//     result.to_string()
// } else {
//     settings.direct_solve_method
// }


	// "direct_solve_method" => 
	//     match typed_value {
	// 	TypedSexp::String(s) => settings.direct_solve_method = (s.to_vec().get(0)).to_string(),
	//         _ => settings.direct_solve_method = settings.direct_solve_method,
	//     },
	// "chordal_decomposition_merge_method" => 
	//     match typed_value {
	// 	TypedSexp::String(s) => settings.chordal_decomposition_merge_method = (s.to_vec().get(0)).to_string(),
	// 	_ => settings.chordal_decomposition_merge_method = settings.chordal_decomposition_merge_method,
	//     },
