async def test_delete_tenant(client, tenant, test_api_key):
    resp = await client.delete(
        f"/api/v1/tenants/{tenant.id}",
        headers={"X-API-Key": test_api_key},
    )
    assert resp.status_code == 204


async def test_delete_config(client, agent_config, test_api_key):
    resp = await client.delete(
        f"/api/v1/configs/{agent_config.id}",
        headers={"X-API-Key": test_api_key},
    )
    assert resp.status_code == 204


async def test_delete_workflow(client, workflow, test_api_key):
    resp = await client.delete(
        f"/api/v1/workflows/{workflow.id}",
        headers={"X-API-Key": test_api_key},
    )
    assert resp.status_code == 204


async def test_delete_user(client, user, test_api_key):
    resp = await client.delete(
        f"/api/v1/users/{user.id}",
        headers={"X-API-Key": test_api_key},
    )
    assert resp.status_code == 204
