# gh #467. Two endpoint tests, copy-pasted. Only the route literal differs:
# same awaited client call, same headers dict, same status assertion — and no
# helper anywhere in this fixture that either copy could have been calling.
# Extracting `delete_and_expect_204(client, route, api_key)` deletes the whole
# second copy, so a reviewer would send this back as duplication to fix.


async def test_delete_order(client, record, test_api_key):
    resp = await client.delete(
        f"/api/v1/orders/{record.id}",
        headers={"X-API-Key": test_api_key},
    )
    assert resp.status_code == 204


async def test_delete_invoice(client, record, test_api_key):
    resp = await client.delete(
        f"/api/v1/invoices/{record.id}",
        headers={"X-API-Key": test_api_key},
    )
    assert resp.status_code == 204
