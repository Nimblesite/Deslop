def match_fields(alpha, beta, gamma, delta):
    head = alpha != beta
    tail = gamma != delta
    both = head != tail
    return both != alpha
