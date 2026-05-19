"""Companion test module so the cluster spans at least two files."""

from dispatcher_support import (
    DispatcherKind,
    MockDispatcher,
    get_dispatcher,
    register_dispatcher,
)


def test_register_sync() -> None:
    mock = MockDispatcher()
    register_dispatcher(DispatcherKind.SYNC, mock)
    assert get_dispatcher(DispatcherKind.SYNC) is mock


def test_register_async() -> None:
    mock = MockDispatcher()
    register_dispatcher(DispatcherKind.ASYNC, mock)
    assert get_dispatcher(DispatcherKind.ASYNC) is mock


def test_register_batch() -> None:
    mock = MockDispatcher()
    register_dispatcher(DispatcherKind.BATCH, mock)
    assert get_dispatcher(DispatcherKind.BATCH) is mock
