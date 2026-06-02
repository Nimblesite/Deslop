"""Workspace / Fly registry configuration."""

# The image namespace controls where built workspace images are pushed.
WORKSPACE_IMAGE_NAMESPACE = "registry.fly.io/nimblesite-workspaces"

# Default tag applied when no explicit version is requested by the caller.
WORKSPACE_IMAGE_DEFAULT_TAG = "latest"

# Registry host used for all push and pull operations against Fly.
FLY_REGISTRY_HOST = "registry.fly.io"

# Org slug scoping every machine and volume created for a workspace.
FLY_ORG_SLUG = "nimblesite"

# Builder image used to assemble the per-workspace runtime layer.
WORKSPACE_BUILDER_IMAGE = "registry.fly.io/nimblesite-builders/base"
