def permit_access(alpha, beta, gamma, delta):
    near = alpha or beta
    far = gamma or delta
    joined = near or far
    return joined or alpha
