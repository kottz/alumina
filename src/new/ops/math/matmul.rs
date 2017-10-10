use new::graph::{GraphDef, NodeID, OpID, PassID, DataID, Storage, GraphShapes, ErrorKind, Result};
use new::ops::{standard_op_name, Op, OpInstance, Pass};
use new::ops::loss::linear::Linear;
use new::shape::{NodeShape, NodeDim};
use ndarray::{IxDyn, Dimension};
use std::any::Any;
use matrixmultiply;

/// Calculate C += α A B
#[derive(Debug, Clone)]
pub struct MatMul {
	name: Option<String>,
	A_id: NodeID,
	B_id: NodeID,
	C_id: NodeID,
	A_trans: bool,
	B_trans: bool,
	C_trans: bool,
	M: Option<usize>,
	N: Option<usize>,
	K: Option<usize>,
	alpha: f32,
}

impl MatMul {
	pub fn new(A_id: &NodeID, B_id: &NodeID, C_id: &NodeID) -> Self{
		MatMul {
			name: None,
			A_id: A_id.clone(),
			B_id: B_id.clone(),
			C_id: C_id.clone(),
			A_trans: false,
			B_trans: false,
			C_trans: false,
			M: None,
			N: None,
			K: None,
			alpha: 1.0,
		}
	}

	pub fn alpha(mut self, alpha: f32) -> Self {
		self.alpha = alpha;
		self
	}

	pub fn a_trans(mut self, trans: bool) -> Self {
		self.A_trans = trans;
		self
	}

	pub fn b_trans(mut self, trans: bool) -> Self {
		self.B_trans = trans;
		self
	}

	pub fn c_trans(mut self, trans: bool) -> Self {
		self.C_trans = trans;
		self
	}

	pub fn m(mut self, m: usize) -> Self {
		self.M = Some(m);
		self
	}
	
	pub fn n(mut self, n: usize) -> Self {
		self.N = Some(n);
		self
	}

	pub fn k(mut self, k: usize) -> Self {
		self.K = Some(k);
		self
	}
}


impl Op for MatMul {
	type InstanceType = MatMulInstance;

	fn type_name(&self) -> &'static str {
		"MatMul"
	}

	fn name<T: Into<String>>(mut self, name: T) -> Self{
		self.name = Some(name.into());
		self
	}

	fn build(self, graph: &mut GraphDef, op_id: &OpID) -> Result<Self::InstanceType> {
		let name = standard_op_name(&self, &self.name, graph, &[self.A_id.clone(), self.B_id.clone()], &[self.C_id.clone()]);
		
		Ok(MatMulInstance{
			name: name,
			A_id: self.A_id.clone(),
			B_id: self.B_id.clone(),
			C_id: self.C_id.clone(),
			A_trans: self.A_trans,
			B_trans: self.A_trans,
			C_trans: self.A_trans,
			M: self.M,
			N: self.N,
			K: self.K,
			forward_id: graph.add_pass(MatMulPass::new(
				self.A_id.value_id(),
				self.B_id.value_id(),
				self.C_id.value_id(),
				self.A_trans,
				self.B_trans,
				self.C_trans,
				self.M,
				self.N,
				self.K,
				self.alpha,
			)),
			backward1_id: graph.add_pass(MatMulPass::new( // B' = At C
				self.A_id.value_id(),
				self.C_id.gradient_id(),
				self.B_id.gradient_id(),
				!self.A_trans,
				self.C_trans,
				self.B_trans,
				self.K, // m = k
				self.N, // n = n
				self.M, // k = m
				self.alpha,
			)),
			backward2_id: graph.add_pass(MatMulPass::new( // A' = C' Bt
				self.C_id.gradient_id(),
				self.B_id.value_id(),
				self.A_id.gradient_id(),
				self.C_trans,
				!self.B_trans,
				self.A_trans,
				self.M, // m = m
				self.K, // n = k
				self.N, // k = n
				self.alpha,
			)),
		})
	}
}


#[derive(Debug, Clone)]
pub struct MatMulInstance {
	name: String,
	A_id: NodeID,
	B_id: NodeID,
	C_id: NodeID,
	A_trans: bool,
	B_trans: bool,
	C_trans: bool,
	M: Option<usize>,
	N: Option<usize>,
	K: Option<usize>,
	forward_id: PassID,
	backward1_id: PassID,
	backward2_id: PassID,
}

impl OpInstance for MatMulInstance {
	fn instance_name(&self) -> &str {&self.name}

	fn dependencies(&self) -> (Vec<NodeID>, Vec<NodeID>){
		(vec![self.A_id.clone(), self.B_id.clone()], vec![self.C_id.clone()])
	}

	fn inner_passes(&self) -> Vec<PassID> {
		vec![self.forward_id.clone(), self.backward1_id.clone(), self.backward2_id.clone()]
	}

	fn inner_ops(&self) -> Vec<OpID> {vec![]}

	fn inner_nodes(&self) -> Vec<NodeID> {vec![]}

	fn propagate_shape_constraints(&self, shapes: &mut GraphShapes) -> Result<()>{

		// Use the shape of A and B to try and inffer a single unknown dimension of C
		// if shape of C has more than 1 unknown then throw error

		// let A_shape = shapes.get_shape(&self.A_id).to_data_shape()?;
		// let B_shape = shapes.get_shape(&self.A_id).to_data_shape()?;
		// let A_shape = A_shape.slice();
		// let B_shape = B_shape.slice();
		
		// #[allow(non_snake_case)]
		// let mut M = self.M.clone();
		// #[allow(non_snake_case)]
		// let mut N = self.N.clone();
		// #[allow(non_snake_case)]
		// let mut K = self.K.clone();

		// // If no matrix dimensions are known, guess based on any of the inputs having 2 dimensions.
		// if let (None, None, None) = (M, N, K){
		// 	if A_shape.len() == 2 {
		// 		if self.A_trans {
		// 			K = Some(A_shape[0]);
		// 			M = Some(A_shape[1]);
		// 		} else {
		// 			M = Some(A_shape[0]);
		// 			K = Some(A_shape[1]);
		// 		}
		// 	} else if B_shape.len() == 2 {
		// 		if self.B_trans {
		// 			N = Some(B_shape[0]);
		// 			K = Some(B_shape[1]);
		// 		} else {
		// 			K = Some(B_shape[0]);
		// 			N = Some(B_shape[1]);
		// 		}
		// 	} else if C_shape.len() == 2 {
		// 		if self.C_trans {
		// 			N = Some(C_shape[0]);
		// 			M = Some(C_shape[1]);
		// 		} else {
		// 			M = Some(C_shape[0]);
		// 			N = Some(C_shape[1]);
		// 		}
		// 	} else {
		// 		return Err("MatMulPass could not infer matrix shapes. M, N, K are all None and no inputs or outputs have 2 dimensions.".to_string());
		// 	}
		// }

		// if M.is_some() {
		// 	let m = M.unwrap();

		// 	let k = get_inner(m, K, A_shape, self.A_trans)?;
		// 	let n = get_inner(m, N, C_shape, self.C_trans)?;
		// 	let _k = get_outer(n, Some(k), B_shape, self.B_trans)?; // check
		// 	return Ok((m, n, k));
		// } else if N.is_some() {
		// 	let n = N.unwrap();

		// 	let k = get_outer(n, K, B_shape, self.B_trans)?;
		// 	let m = get_outer(n, M, C_shape, self.C_trans)?;
		// 	let _k = get_inner(m, Some(k), A_shape, self.A_trans)?; // check
		// 	return Ok((m, n, k));
		// } else if K.is_some() {
		// 	let k = K.unwrap();

		// 	let m = get_outer(k, M, A_shape, self.A_trans)?;
		// 	let n = get_inner(k, N, B_shape, self.B_trans)?;
		// 	let _n = get_inner(m, N, C_shape, self.C_trans)?; // check
		// 	return Ok((m, n, k));
		// } else {
		// 	unreachable!();
		// }
		
		Ok(())
	}
}


/// Calculate C += α A B
///
/// If one or more of M, N, or K are known the others can be found at run time.
/// If none of M, N or K are known, a guess will be made based on any argumets having 2 dimensions.
/// Shapes may be n-dimensional, however they must split cleanly along an axis into M, N, K shapes.
/// An error will be returned if any inconsistencies are found.
#[derive(Debug, Clone)]
pub struct MatMulPass{
	mat_A: DataID,
	mat_B: DataID,
	mat_C: DataID,
	A_trans: bool,
	B_trans: bool,
	C_trans: bool,
	M: Option<usize>,
	N: Option<usize>,
	K: Option<usize>,
	alpha: f32,
}

impl MatMulPass {
	pub fn new(mat_A: DataID,
			mat_B: DataID,
			mat_C: DataID,
			A_trans: bool,
			B_trans: bool,
			C_trans: bool,
			M: Option<usize>,
			N: Option<usize>,
			K: Option<usize>,
			alpha: f32) -> Self {
		MatMulPass {
			mat_A: mat_A,
			mat_B: mat_B,
			mat_C: mat_C,
			A_trans: A_trans,
			B_trans: B_trans,
			C_trans: C_trans,
			M: M,
			N: N,
			K: K,
			alpha: alpha,
		}
	}

	/// If one or more of M, N, or K are known find the others
	/// Shapes may be multidimensional, however they must split cleanly along an axis into M, N, K shapes.
	/// An error will be returned if any inconsistencies are found.
	fn find_mnk(&self, A_shape: &[usize],
				B_shape: &[usize],
				C_shape: &[usize]) -> ::std::result::Result<(usize, usize, usize), String>{

		#[allow(non_snake_case)]
		let mut M = self.M.clone();
		#[allow(non_snake_case)]
		let mut N = self.N.clone();
		#[allow(non_snake_case)]
		let mut K = self.K.clone();

		// If no matrix dimensions are known, guess based on any of the inputs having 2 dimensions.
		if let (None, None, None) = (M, N, K){
			if A_shape.len() == 2 {
				if self.A_trans {
					K = Some(A_shape[0]);
					M = Some(A_shape[1]);
				} else {
					M = Some(A_shape[0]);
					K = Some(A_shape[1]);
				}
			} else if B_shape.len() == 2 {
				if self.B_trans {
					N = Some(B_shape[0]);
					K = Some(B_shape[1]);
				} else {
					K = Some(B_shape[0]);
					N = Some(B_shape[1]);
				}
			} else if C_shape.len() == 2 {
				if self.C_trans {
					N = Some(C_shape[0]);
					M = Some(C_shape[1]);
				} else {
					M = Some(C_shape[0]);
					N = Some(C_shape[1]);
				}
			} else {
				return Err("MatMulPass could not infer matrix shapes. M, N, K are all None and no inputs or outputs have 2 dimensions.".to_string());
			}
		}

		if M.is_some() {
			let m = M.unwrap();

			let k = get_inner(m, K, A_shape, self.A_trans)?;
			let n = get_inner(m, N, C_shape, self.C_trans)?;
			let _k = get_outer(n, Some(k), B_shape, self.B_trans)?; // check
			return Ok((m, n, k));
		} else if N.is_some() {
			let n = N.unwrap();

			let k = get_outer(n, K, B_shape, self.B_trans)?;
			let m = get_outer(n, M, C_shape, self.C_trans)?;
			let _k = get_inner(m, Some(k), A_shape, self.A_trans)?; // check
			return Ok((m, n, k));
		} else if K.is_some() {
			let k = K.unwrap();

			let m = get_outer(k, M, A_shape, self.A_trans)?;
			let n = get_inner(k, N, B_shape, self.B_trans)?;
			let _n = get_inner(m, Some(n), C_shape, self.C_trans)?; // check
			return Ok((m, n, k));
		} else {
			unreachable!();
		}
	}
}

impl Pass for MatMulPass {
	fn type_name(&self) -> &'static str {"MatMulPass"}

	fn dependencies(&self) -> (Vec<DataID>, Vec<DataID>){
		(vec![self.mat_A.clone(), self.mat_B.clone()],
		vec![self.mat_C.clone()])
	}

	fn run (&self, data: &Storage) -> Result<Box<Any>>{
		let mat_A = data.get(&self.mat_A)?;
		let mat_B = data.get(&self.mat_B)?;
		let mut mat_C = data.get_mut(&self.mat_C)?;

		let (m, n, k) = match self.find_mnk(mat_A.shape(), mat_B.shape(), mat_C.shape()){
			Err(message) => bail!(ErrorKind::ForwardPassError(format!("Error in '{}' while finding M, N and K: {}", data.graph().pass_name(data.get_current_pass().as_ref().unwrap()), message))),
			Ok(x) => x,
		};

		let mat_A = mat_A.as_slice().unwrap();
		let mat_B = mat_B.as_slice().unwrap();
		let mat_C = mat_C.as_slice_mut().unwrap();
		
		unsafe{
			let (rsa, csa) = if self.A_trans{(1, m)} else {(k, 1)};
			let (rsb, csb) = if self.B_trans{(1, k)} else {(n, 1)};
			let (rsc, csc) = if self.C_trans{(1, m)} else {(n, 1)};

			matrixmultiply::sgemm(m, k, n,
				self.alpha,
				mat_A.as_ptr(), rsa as isize, csa as isize,
				mat_B.as_ptr(), rsb as isize, csb as isize,
				1.0,
				mat_C.as_mut_ptr(), rsc as isize, csc as isize,);
		}

		Ok(Box::new(()))
	}
}


fn get_inner(outer: usize, inner: Option<usize>, shape: &[usize], trans: bool) -> ::std::result::Result<usize, String> {

	if trans {
		return get_outer(outer, inner, shape, false);
	};

	let (i, o) = shape.iter().fold((1, 1), |(mut i, mut o), &dim|{
		if o == outer {
			i *= dim;
		} else {
			o *= dim;
		}
		(i, o)
	});

	if o != outer {
		return Err(format!("Could not determine inner (outer if transposed) dimension of matrix. Outer matrix dimension: '{}' did not equal the product of outer dimensions in shape{:?}", outer, shape))
	}


	match inner {
		None => Ok(i),
		Some(existing) if existing == i => Ok(i),
		_ => Err(format!("Could not determine inner (outer if transposed) dimension of matrix. The found inner dimension: '{}' conflicts with the inner dimension hint: '{:?}'", i, inner))
	}
}

fn get_outer(inner: usize, outer: Option<usize>, shape: &[usize], trans: bool) -> ::std::result::Result<usize, String> {

	if trans {
		return get_inner(inner, outer, shape, false);
	};

	let (i, o) = shape.iter().rev().fold((1, 1), |(mut i, mut o), &dim|{
		if i == inner {
			o *= dim;
		} else {
			i *= dim;
		}
		(i, o)
	});

	if i != inner {
		return Err(format!("Could not determine outer (inner if transposed) dimension of matrix. Inner matrix dimension: '{}' did not equal the product of inner dimensions in shape{:?}", inner, shape))
	}

	match outer {
		None => Ok(o),
		Some(existing) if existing == o => Ok(o),
		_ => Err(format!("Could not determine outer (inner if transposed) dimension of matrix. The found outer dimension: '{}' conflicts with the outer dimension hint: '{:?}'", o, outer))
	}
}


#[test]
fn test_matmul_backprop(){
	_matmul_backprop().unwrap();
}

fn _matmul_backprop() -> Result<()>{
	use new::graph::GraphDef;
	use new::ops::numeric_check::numeric_test;
	use new::ops::loss::mse::Mse;
	use ordermap::OrderMap;

	let mut g = GraphDef::new();

	let node1 = g.new_node(shape![7, 5], "input1", tag![])?;
	let node2 = g.new_node(shape![5, 16], "input2", tag![])?;
	let node3 = g.new_node(shape![7, 16], "output", tag![])?;
	let node4 = g.new_node(shape![7, 16], "target", tag![])?;

	let _o1 = g.new_op(MatMul::new(&node1, &node2, &node3), tag![])?;
	let _o2 = g.new_op(Mse::new(&node3, &node4), tag![])?;

	let iters = 100;
	let failures = 1;
	let tolerance = 0.001;
	let step_size = 1E-2;
	let default_variance = 1.0;
	numeric_test(iters, failures, tolerance, &g, step_size, default_variance, &mut OrderMap::new())?;

	Ok(())
}

#[test]
fn test_matmul_a_trans_backprop(){
	_matmul_a_trans_backprop().unwrap();
}

fn _matmul_a_trans_backprop() -> Result<()>{
	use new::graph::GraphDef;
	use new::ops::numeric_check::numeric_test;
	use new::ops::loss::mse::Mse;
	use ordermap::OrderMap;

	let mut g = GraphDef::new();

	let node1 = g.new_node(shape![5, 7], "input1", tag![])?;
	let node2 = g.new_node(shape![5, 16], "input2", tag![])?;
	let node3 = g.new_node(shape![7, 16], "output", tag![])?;
	let node4 = g.new_node(shape![7, 16], "target", tag![])?;

	let _o1 = g.new_op(MatMul::new(&node1, &node2, &node3).a_trans(true), tag![])?;
	let _o2 = g.new_op(Mse::new(&node3, &node4), tag![])?;

	let iters = 100;
	let failures = 1;
	let tolerance = 0.001;
	let step_size = 1E-2;
	let default_variance = 1.0;
	numeric_test(iters, failures, tolerance, &g, step_size, default_variance, &mut OrderMap::new())?;

	Ok(())
}

#[test]
fn test_matmul_c_trans_backprop(){
	_matmul_c_trans_backprop().unwrap();
}

fn _matmul_c_trans_backprop() -> Result<()>{
	use new::graph::GraphDef;
	use new::ops::numeric_check::numeric_test;
	use new::ops::loss::mse::Mse;
	use ordermap::OrderMap;

	let mut g = GraphDef::new();

	let node1 = g.new_node(shape![7, 5], "input1", tag![])?;
	let node2 = g.new_node(shape![5, 16], "input2", tag![])?;
	let node3 = g.new_node(shape![16, 7], "output", tag![])?;
	let node4 = g.new_node(shape![16, 7], "target", tag![])?;

	let _o1 = g.new_op(MatMul::new(&node1, &node2, &node3).c_trans(true), tag![])?;
	let _o2 = g.new_op(Mse::new(&node3, &node4), tag![])?;

	let iters = 100;
	let failures = 1;
	let tolerance = 0.002;
	let step_size = 1E-2;
	let default_variance = 1.0;
	numeric_test(iters, failures, tolerance, &g, step_size, default_variance, &mut OrderMap::new())?;

	Ok(())
}