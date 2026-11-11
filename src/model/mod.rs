use anyhow::Result;
use ndarray::{Array1, Array2};
use rand::Rng;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelState {
    pub w1: Vec<Vec<f32>>,
    pub b1: Vec<f32>,
    pub w2: Vec<Vec<f32>>,
    pub b2: Vec<f32>,
    pub v_w1: Vec<Vec<f32>>,
    pub v_b1: Vec<f32>,
    pub v_w2: Vec<Vec<f32>>,
    pub v_b2: Vec<f32>,
    pub step: u32,
    pub lr: f32,
    pub activation: String,
    pub last_loss: Option<f32>,
}

#[derive(Debug, Serialize)]
pub struct ModelStatus {
    pub initialized: bool,
    pub hidden: usize,
    pub activation: String,
    pub step: u32,
    pub last_loss: Option<f32>,
    pub allocated: usize,
    pub committed: usize,
    pub backend: String,
    pub gpu: String,
    pub gpu_reason: String,
    pub rank: RankInfo,
}

#[derive(Debug, Default, Serialize)]
pub struct RankInfo {
    pub total_entities: usize,
    pub total_relations: usize,
    pub total_observations: usize,
    pub total_facts: usize,
    pub total_tasks: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Activation {
    Sigmoid,
    Tanh,
    LeakyRelu,
}

impl Activation {
    #[inline(always)]
    pub fn from_name(s: &str) -> Self {
        match s {
            "sigmoid" => Activation::Sigmoid,
            "tanh" => Activation::Tanh,
            _ => Activation::LeakyRelu,
        }
    }

    #[inline(always)]
    pub fn as_str(self) -> &'static str {
        match self {
            Activation::Sigmoid => "sigmoid",
            Activation::Tanh => "tanh",
            Activation::LeakyRelu => "leaky",
        }
    }
}

impl Serialize for Activation {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for Activation {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(Activation::from_name(&s))
    }
}

#[derive(Debug)]
pub struct NeuralModel {
    // Layer 1: input(2) -> hidden
    pub w1: Array2<f32>, // shape (H, 2) - row-major: w1[h, i]
    pub b1: Array1<f32>, // shape (H,)
    // Layer 2: hidden -> output(1)
    pub w2: Array2<f32>, // shape (1, H) - row-major: w2[0, h]
    pub b2: Array1<f32>, // shape (1,)
    // Momentum velocities
    pub v_w1: Array2<f32>,
    pub v_b1: Array1<f32>,
    pub v_w2: Array2<f32>,
    pub v_b2: Array1<f32>,
    // Config
    pub lr: f32,
    pub mu: f32,
    pub activation: Activation,
    pub step: u32,
    pub last_loss: Option<f32>,
}

impl NeuralModel {
    pub fn new(
        input_dim: usize,
        hidden: usize,
        output_dim: usize,
        lr: f32,
        activation: &str,
    ) -> Result<Self> {
        // Xavier initialization matching FASM: scale = sqrt(2/(fan_in+fan_out))
        let scale1 = (2.0 / (input_dim + hidden) as f32).sqrt();
        let scale2 = (2.0 / (hidden + output_dim) as f32).sqrt();

        let mut rng = rand::thread_rng();

        let w1 = Array2::from_shape_fn((hidden, input_dim), |_| rng.gen_range(-scale1..scale1));
        let b1 = Array1::zeros(hidden);
        let w2 = Array2::from_shape_fn((output_dim, hidden), |_| rng.gen_range(-scale2..scale2));
        let b2 = Array1::zeros(output_dim);

        Ok(Self {
            w1,
            b1,
            w2,
            b2,
            v_w1: Array2::zeros((hidden, input_dim)),
            v_b1: Array1::zeros(hidden),
            v_w2: Array2::zeros((output_dim, hidden)),
            v_b2: Array1::zeros(output_dim),
            lr,
            mu: 0.9,
            activation: Activation::from_name(activation),
            step: 0,
            last_loss: None,
        })
    }

    pub fn init_weights(&mut self) {
        let scale1 = (2.0 / (self.w1.ncols() + self.w1.nrows()) as f32).sqrt();
        let scale2 = (2.0 / (self.w2.ncols() + self.w2.nrows()) as f32).sqrt();

        let mut rng = rand::thread_rng();

        self.w1.mapv_inplace(|_| rng.gen_range(-scale1..scale1));
        self.b1.fill(0.0);
        self.w2.mapv_inplace(|_| rng.gen_range(-scale2..scale2));
        self.b2.fill(0.0);

        self.v_w1.fill(0.0);
        self.v_b1.fill(0.0);
        self.v_w2.fill(0.0);
        self.v_b2.fill(0.0);

        self.step = 0;
        self.last_loss = None;
    }

    #[inline(always)]
    fn leaky_relu(x: f32) -> f32 {
        if x > 0.0 {
            x
        } else {
            0.01 * x
        }
    }

    #[inline(always)]
    fn leaky_relu_derivative(z_plus_b1: f32) -> f32 {
        if z_plus_b1 > 0.0 {
            1.0
        } else {
            0.01
        }
    }

    #[inline(always)]
    fn sigmoid(x: f32) -> f32 {
        1.0 / (1.0 + (-x).exp())
    }

    #[inline(always)]
    fn sigmoid_derivative(z_plus_b1: f32) -> f32 {
        let s = Self::sigmoid(z_plus_b1);
        s * (1.0 - s)
    }

    #[inline(always)]
    fn tanh(x: f32) -> f32 {
        x.tanh()
    }

    #[inline(always)]
    fn tanh_derivative(z_plus_b1: f32) -> f32 {
        let t = z_plus_b1.tanh();
        1.0 - t * t
    }

    #[inline(always)]
    fn activate(&self, z_plus_b1: f32) -> f32 {
        match self.activation {
            Activation::Sigmoid => Self::sigmoid(z_plus_b1),
            Activation::Tanh => Self::tanh(z_plus_b1),
            Activation::LeakyRelu => Self::leaky_relu(z_plus_b1),
        }
    }

    #[inline(always)]
    fn activate_derivative(&self, z_plus_b1: f32) -> f32 {
        match self.activation {
            Activation::Sigmoid => Self::sigmoid_derivative(z_plus_b1),
            Activation::Tanh => Self::tanh_derivative(z_plus_b1),
            Activation::LeakyRelu => Self::leaky_relu_derivative(z_plus_b1),
        }
    }

    /// Forward pass matching FASM fwd_cache:
    /// Z1 = X @ W1^T (but W1 is Hx2, X is Nx2 -> Z1 is NxH)
    /// A1 = activation(Z1 + b1_broadcast)
    /// Y = A1 @ W2^T + b2  (W2 is 1xH, A1 is NxH -> Y is Nx1)
    pub fn forward(&self, x: &Array1<f32>) -> Array1<f32> {
        // x: (2,), W1: (H, 2) -> h: (H,)
        let z1 = self.w1.dot(x) + &self.b1;
        let a1 = z1.mapv(|z| self.activate(z));
        // a1: (H,), W2: (1, H) -> y: (1,)

        self.w2.dot(&a1) + &self.b2
    }

    pub fn predict(&self, xs: &[f32], ys: &[f32]) -> Vec<f32> {
        let n = xs.len().min(ys.len());
        let mut results = Vec::with_capacity(n);
        let mut x = Array1::zeros(2);

        for i in 0..n {
            x[0] = xs[i];
            x[1] = ys[i];
            let y = self.forward(&x);
            results.push(y[0]);
        }
        results
    }

    /// Full-batch training matching FASM exactly:
    /// 1. Forward: Z1 = X @ W1^T, A1 = leaky_relu(Z1 + b1), Y = A1 @ W2^T + b2
    /// 2. Loss: MSE = mean((Y - T)^2)
    /// 3. Backward: dy = (Y - T) * 2/N
    ///    dW2 = A1^T @ dy, db2 = sum(dy)
    ///    For each h: da = dy * W2[h], dz = da * (Z1+b1 > 0 ? 1 : 0.01)
    ///    db1 += dz, dW1[:,h] += dz * X^T
    /// 4. SGD with momentum: v = mu*v - lr*g; p += v
    pub fn train_batch(
        &mut self,
        xs: &[f32],
        ys: &[f32],
        targets: &[f32],
        steps: u32,
    ) -> Result<f32> {
        let n = xs.len().min(ys.len()).min(targets.len());
        if n == 0 {
            return Ok(0.0);
        }

        let h = self.w1.nrows(); // hidden size
        let mut last_loss = 0.0;

        // Pre-allocate buffers (matching FASM z1buf, a1buf, ybuf, gw1buf, gb1buf, gw2buf, gb2buf)
        let mut z1: Array2<f32>; // Z1[N][H] - assigned per-iteration below
        let mut a1 = Array2::zeros((n, h)); // A1[N][H]
        let mut y = Array1::zeros(n); // Y[N]
        let mut dy = Array1::zeros(n); // dy[N]

        let mut gw1 = Array2::zeros((h, 2)); // GW1[H][2]
        let mut gb1 = Array1::zeros(h); // Gb1[H]
        let mut gw2 = Array2::zeros((1, h)); // GW2[1][H]
        let mut gb2 = Array1::zeros(1); // Gb2[1]

        // Input matrix X[N][2]
        let mut x_mat = Array2::zeros((n, 2));
        for i in 0..n {
            x_mat[[i, 0]] = xs[i];
            x_mat[[i, 1]] = ys[i];
        }

        let targets_arr = Array1::from_vec(targets[..n].to_vec());

        for _ in 0..steps {
            // Clear gradient buffers
            gw1.fill(0.0);
            gb1.fill(0.0);
            gw2.fill(0.0);
            gb2.fill(0.0);

            // ===== FORWARD PASS =====
            // Z1 = X @ W1^T  -> (N, 2) @ (2, H) = (N, H)
            // W1 is (H, 2), we need W1^T which is (2, H)
            // z1 = x_mat @ w1.t()
            z1 = x_mat.dot(&self.w1.t());

            // A1 = activation(Z1 + b1_broadcast)
            for i in 0..n {
                for j in 0..h {
                    let z_plus_b1 = z1[[i, j]] + self.b1[j];
                    a1[[i, j]] = self.activate(z_plus_b1);
                }
            }

            // Y = A1 @ W2^T + b2  -> (N, H) @ (H, 1) = (N, 1)
            // W2 is (1, H), W2^T is (H, 1)
            // y = a1 @ w2.t() -> broadcasts correctly
            for i in 0..n {
                let mut sum = self.b2[0];
                for j in 0..h {
                    sum += a1[[i, j]] * self.w2[[0, j]];
                }
                y[i] = sum;
            }

            // Loss = mean((Y - T)^2)
            let mut total_loss = 0.0;
            for i in 0..n {
                let err = y[i] - targets_arr[i];
                dy[i] = err * (2.0 / n as f32); // dy = (Y - T) * 2/N
                total_loss += err * err;
            }
            total_loss /= n as f32;

            // ===== BACKWARD PASS =====
            // dW2 = A1^T @ dy  -> (H, N) @ (N,) = (H,) -> reshape to (1, H)
            for j in 0..h {
                let mut sum = 0.0;
                for i in 0..n {
                    sum += a1[[i, j]] * dy[i];
                }
                gw2[[0, j]] = sum;
            }
            gb2[0] = dy.sum();

            // dW1, db1
            // For each hidden unit h: da = dy * W2[h]; dz = da * activation'(z1 + b1)
            for j in 0..h {
                let w2_h = self.w2[[0, j]];
                let mut db1_j = 0.0;
                let mut dw1_j0 = 0.0;
                let mut dw1_j1 = 0.0;

                for i in 0..n {
                    let da = dy[i] * w2_h;
                    // activation derivative uses PRE-BIAS sum (z1 + b1) matching FASM
                    let z_plus_b1 = z1[[i, j]] + self.b1[j];
                    let dz = da * self.activate_derivative(z_plus_b1);

                    db1_j += dz;
                    dw1_j0 += dz * x_mat[[i, 0]];
                    dw1_j1 += dz * x_mat[[i, 1]];
                }

                gb1[j] = db1_j;
                gw1[[j, 0]] = dw1_j0;
                gw1[[j, 1]] = dw1_j1;
            }

            // ===== SGD WITH MOMENTUM =====
            // v = mu * v - lr * g; p += v
            let lr = self.lr;
            let mu = self.mu;

            // W1 (H, 2)
            self.v_w1 = mu * &self.v_w1 - lr * &gw1;
            self.w1 += &self.v_w1;

            // b1 (H,)
            self.v_b1 = mu * &self.v_b1 - lr * &gb1;
            self.b1 += &self.v_b1;

            // W2 (1, H)
            self.v_w2 = mu * &self.v_w2 - lr * &gw2;
            self.w2 += &self.v_w2;

            // b2 (1,)
            self.v_b2 = mu * &self.v_b2 - lr * &gb2;
            self.b2 += &self.v_b2;

            self.step += 1;
            last_loss = total_loss;
        }

        self.last_loss = Some(last_loss);
        Ok(last_loss)
    }

    pub fn to_state(&self) -> ModelState {
        ModelState {
            w1: self.w1.t().rows().into_iter().map(|r| r.to_vec()).collect(),
            b1: self.b1.to_vec(),
            w2: self.w2.t().rows().into_iter().map(|r| r.to_vec()).collect(),
            b2: self.b2.to_vec(),
            v_w1: self
                .v_w1
                .t()
                .rows()
                .into_iter()
                .map(|r| r.to_vec())
                .collect(),
            v_b1: self.v_b1.to_vec(),
            v_w2: self
                .v_w2
                .t()
                .rows()
                .into_iter()
                .map(|r| r.to_vec())
                .collect(),
            v_b2: self.v_b2.to_vec(),
            step: self.step,
            lr: self.lr,
            activation: self.activation.as_str().to_string(),
            last_loss: self.last_loss,
        }
    }

    pub fn from_state(state: ModelState) -> Result<Self> {
        let w1 = Array2::from_shape_vec(
            (state.w1[0].len(), state.w1.len()),
            state.w1.into_iter().flatten().collect(),
        )?
        .t()
        .to_owned();

        let b1 = Array1::from_vec(state.b1);

        let w2 = Array2::from_shape_vec(
            (state.w2[0].len(), state.w2.len()),
            state.w2.into_iter().flatten().collect(),
        )?
        .t()
        .to_owned();

        let b2 = Array1::from_vec(state.b2);

        let v_w1 = Array2::from_shape_vec(
            (state.v_w1[0].len(), state.v_w1.len()),
            state.v_w1.into_iter().flatten().collect(),
        )?
        .t()
        .to_owned();

        let v_b1 = Array1::from_vec(state.v_b1);

        let v_w2 = Array2::from_shape_vec(
            (state.v_w2[0].len(), state.v_w2.len()),
            state.v_w2.into_iter().flatten().collect(),
        )?
        .t()
        .to_owned();

        let v_b2 = Array1::from_vec(state.v_b2);

        Ok(Self {
            w1,
            b1,
            w2,
            b2,
            v_w1,
            v_b1,
            v_w2,
            v_b2,
            lr: state.lr,
            mu: 0.9,
            activation: Activation::from_name(&state.activation),
            step: state.step,
            last_loss: state.last_loss,
        })
    }

    pub fn save(&self, path: &str) -> Result<()> {
        let state = self.to_state();
        let json = serde_json::to_string_pretty(&state)?;
        fs::write(path, json)?;
        Ok(())
    }

    pub fn load(path: &str) -> Result<Self> {
        let json = fs::read_to_string(path)?;
        let state: ModelState = serde_json::from_str(&json)?;
        Self::from_state(state)
    }

    pub fn status(&self) -> ModelStatus {
        let allocated = self.w1.len() + self.b1.len() + self.w2.len() + self.b2.len();

        ModelStatus {
            initialized: true,
            hidden: self.w1.nrows(),
            activation: self.activation.as_str().to_string(),
            step: self.step,
            last_loss: self.last_loss,
            allocated,
            committed: allocated,
            backend: "pure-rust".to_string(),
            gpu: "unavailable".to_string(),
            gpu_reason: "no CUDA; CPU reference is the supported backend".to_string(),
            rank: RankInfo::default(),
        }
    }
}
