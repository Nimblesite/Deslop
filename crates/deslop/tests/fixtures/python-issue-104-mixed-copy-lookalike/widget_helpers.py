import router


def build_url(host):
    joined = router.join(host)
    return joined


def split_path(text):
    parts = router.split(text)
    return parts
