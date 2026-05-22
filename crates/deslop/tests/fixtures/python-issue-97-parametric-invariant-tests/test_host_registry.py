"""Pytest invariants across each variant of an enum discriminator.

Each test verifies that `register_host(KIND, mock)` followed by
`get_host(KIND)` returns the same mock. After identifier normalisation
the bodies all become structurally and tokenwise identical — the only
varying token is the enum member access `AgentWorkspaceHostKind.K8S`
vs `AgentWorkspaceHostKind.DOCKER` etc. Each test asserts a distinct
spec point: merging them into one parametrised test is the right
refactor in some shops but is intentionally rejected here because
each test name appears verbatim in coverage dashboards and CI titles.
"""

from host_support import (
    AgentWorkspaceHostKind,
    MockAgentWorkspaceHost,
    get_host,
    register_host,
)


def test_register_k8s() -> None:
    mock = MockAgentWorkspaceHost()
    register_host(AgentWorkspaceHostKind.K8S, mock)
    assert get_host(AgentWorkspaceHostKind.K8S) is mock


def test_register_docker() -> None:
    mock = MockAgentWorkspaceHost()
    register_host(AgentWorkspaceHostKind.DOCKER, mock)
    assert get_host(AgentWorkspaceHostKind.DOCKER) is mock


def test_register_podman() -> None:
    mock = MockAgentWorkspaceHost()
    register_host(AgentWorkspaceHostKind.PODMAN, mock)
    assert get_host(AgentWorkspaceHostKind.PODMAN) is mock
