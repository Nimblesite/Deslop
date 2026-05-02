namespace AiCms.Api.Endpoints;

public static class ChatEndpoints
{
    public static void MapChat(WebApplication app, bool requireAuth)
    {
        var chat = app.MapGroup("/api/sites/{siteId}/chat");
        if (requireAuth)
            chat.RequireAuthorization();
        chat.MapPost("", HandleChatAsync).RequireRateLimiting("chat");
        chat.MapGet("history", HandleHistoryAsync).RequireRateLimiting("chat");
        chat.MapDelete("session", HandleEndSessionAsync).RequireRateLimiting("chat");
    }

    private static Task HandleChatAsync() => Task.CompletedTask;
    private static Task HandleHistoryAsync() => Task.CompletedTask;
    private static Task HandleEndSessionAsync() => Task.CompletedTask;
}
