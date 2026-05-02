namespace AiCms.Api.Endpoints;

public static class SiteEndpoints
{
    public static void MapSites(WebApplication app, bool requireAuth)
    {
        var sites = app.MapGroup("/api/sites");
        if (requireAuth)
            sites.RequireAuthorization();
        sites.MapPost("", HandleGenerateAsync).RequireRateLimiting("create");
        sites.MapGet("list", HandleListAsync).RequireRateLimiting("create");
        sites.MapDelete("entry", HandleRemoveAsync).RequireRateLimiting("create");
    }

    private static Task HandleGenerateAsync() => Task.CompletedTask;
    private static Task HandleListAsync() => Task.CompletedTask;
    private static Task HandleRemoveAsync() => Task.CompletedTask;
}
