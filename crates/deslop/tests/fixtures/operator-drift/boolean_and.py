def permit_access(alpha, beta, gamma, delta):
    near = alpha and beta
    far = gamma and delta
    joined = near and far
    return joined and alpha
