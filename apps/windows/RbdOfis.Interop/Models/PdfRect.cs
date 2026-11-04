using System;
using System.Text.Json.Serialization;

namespace RbdOfis.Interop.Models;

/// <summary>
/// A bounding rectangle in PDF point coordinates (72 points = 1 inch).
/// Origin (0,0) is typically at bottom-left in standard PDF, or top-left in normalized view.
/// </summary>
public record PdfRect(
    [property: JsonPropertyName("x0")] float X0,
    [property: JsonPropertyName("y0")] float Y0,
    [property: JsonPropertyName("x1")] float X1,
    [property: JsonPropertyName("y1")] float Y1)
{
    [JsonIgnore]
    public float Width => Math.Abs(X1 - X0);

    [JsonIgnore]
    public float Height => Math.Abs(Y1 - Y0);

    [JsonIgnore]
    public float Left => Math.Min(X0, X1);

    [JsonIgnore]
    public float Top => Math.Min(Y0, Y1);

    public static readonly PdfRect Empty = new(0f, 0f, 0f, 0f);

    public bool Contains(float x, float y)
    {
        float minX = Math.Min(X0, X1);
        float maxX = Math.Max(X0, X1);
        float minY = Math.Min(Y0, Y1);
        float maxY = Math.Max(Y0, Y1);
        return x >= minX && x <= maxX && y >= minY && y <= maxY;
    }
}
