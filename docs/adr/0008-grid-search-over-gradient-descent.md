ADR-0008: Grid Search Over Gradient Descent for the Parameter Optimizer
Date: 2025-06-10
Status: Accepted
Context
The position optimizer needs to find the fee tier and range width combination that maximizes the projected fee-to-LVR ratio given realized volatility. This is an optimization problem over a small, discrete search space: 4 fee tiers × approximately 15 range widths = 60 combinations.
Decision
We will use exhaustive grid search — evaluate all 60 combinations and return the maximum. No gradient-based optimization. No heuristic search.
Alternatives Considered
Gradient descent / Adam optimizer: Treats the objective as a continuous differentiable function and follows the gradient toward the maximum. Overkill for 60 evaluation points. Gradient descent also requires the objective function to be smooth and differentiable — the fee tier parameter is discrete, which breaks standard gradient computation. Rejected.
Bayesian optimization: Builds a probabilistic model of the objective function and selects evaluation points intelligently. Appropriate for expensive-to-evaluate objectives with large search spaces. Our objective function evaluates in microseconds and the search space has 60 points. Using Bayesian optimization here is like hiring a logistics consultant to decide which of two routes to take to work. Rejected.
Random search with restarts: Randomly sample parameter combinations and restart from the best found. Introduces non-determinism — two runs on the same data produce different recommendations. Rejected. The optimizer must be deterministic and reproducible.
Analytical solution from the Cartea paper: The Cartea et al. 2022 paper derives a closed-form optimal range under specific distributional assumptions. Could replace the numerical search entirely. Rejected for version one because the closed-form solution requires assumptions (stationary volatility, specific price process) that may not hold for the analysis period. The grid search makes no distributional assumptions — it uses the empirically observed volatility directly.
Consequences
Positive: Completely deterministic. Trivially parallelizable if needed. Easy to debug — you can print the full grid. Zero dependencies beyond standard Rust. Completes in under 1ms.
Negative: Cannot optimize over continuous range widths — resolution is limited to the defined grid steps. Optimal range might fall between two grid points.
Risks: None significant at this scale. If the search space grows substantially in future versions, the grid search can be replaced without changing the surrounding code.
