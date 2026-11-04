using System;
using System.Collections.Generic;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace RbdOfis.Interop.Models;

/// <summary>
/// Custom converter to serialize byte[] as a JSON array of integers [r, g, b, a] rather than base64 string,
/// matching Rust serde's deserialization of `[u8; 4]`.
/// </summary>
public sealed class ByteArrayAsIntArrayJsonConverter : JsonConverter<byte[]>
{
    public override byte[] Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
    {
        if (reader.TokenType == JsonTokenType.String)
        {
            return Convert.FromBase64String(reader.GetString() ?? string.Empty);
        }
        var list = new List<byte>();
        if (reader.TokenType == JsonTokenType.StartArray)
        {
            while (reader.Read() && reader.TokenType != JsonTokenType.EndArray)
            {
                list.Add(reader.GetByte());
            }
        }
        return list.ToArray();
    }

    public override void Write(Utf8JsonWriter writer, byte[] value, JsonSerializerOptions options)
    {
        writer.WriteStartArray();
        foreach (byte b in value)
        {
            writer.WriteNumberValue(b);
        }
        writer.WriteEndArray();
    }
}

/// <summary>
/// High-level Annotation Data Transfer Object matching rbd-engine / rbd-core Annotation model.
/// </summary>
public sealed class AnnotationDto
{
    [JsonPropertyName("id")]
    public ulong Id { get; set; }

    [JsonPropertyName("page_index")]
    public uint PageIndex { get; set; }

    [JsonPropertyName("kind")]
    public object Kind { get; set; } = "Highlight";

    [JsonPropertyName("rect")]
    public PdfRect Rect { get; set; } = PdfRect.Empty;

    [JsonPropertyName("color_rgba")]
    [JsonConverter(typeof(ByteArrayAsIntArrayJsonConverter))]
    public byte[] ColorRgba { get; set; } = [255, 235, 59, 255]; // Yellow default

    [JsonPropertyName("contents")]
    public string Contents { get; set; } = string.Empty;

    [JsonIgnore]
    public string KindName
    {
        get
        {
            if (Kind is string s) return s;
            if (Kind is System.Collections.IDictionary dict)
            {
                foreach (var k in dict.Keys) return k?.ToString() ?? "Highlight";
            }
            if (Kind is JsonElement elem)
            {
                if (elem.ValueKind == JsonValueKind.String) return elem.GetString() ?? "Highlight";
                if (elem.ValueKind == JsonValueKind.Object)
                {
                    foreach (var prop in elem.EnumerateObject())
                    {
                        return prop.Name;
                    }
                }
            }
            return Kind?.ToString() ?? "Highlight";
        }
    }

    [JsonIgnore]
    public List<(float X, float Y)> Points { get; set; } = new();

    [JsonIgnore]
    public float StrokeWidth { get; set; } = 2.0f;

    [JsonIgnore]
    public float FontSize { get; set; } = 12.0f;

    public static AnnotationDto CreateHighlight(ulong id, uint pageIndex, PdfRect rect, byte[]? color = null)
    {
        return new AnnotationDto
        {
            Id = id,
            PageIndex = pageIndex,
            Kind = "Highlight",
            Rect = rect,
            ColorRgba = color ?? [255, 235, 59, 128], // semi-transparent yellow
            Contents = string.Empty
        };
    }

    public static AnnotationDto CreateRectangle(ulong id, uint pageIndex, PdfRect rect, byte[]? color = null)
    {
        return new AnnotationDto
        {
            Id = id,
            PageIndex = pageIndex,
            Kind = "Rectangle",
            Rect = rect,
            ColorRgba = color ?? [230, 38, 0, 255], // Red border
            Contents = string.Empty
        };
    }

    public static AnnotationDto CreateStickyNote(ulong id, uint pageIndex, PdfRect rect, string text, byte[]? color = null)
    {
        return new AnnotationDto
        {
            Id = id,
            PageIndex = pageIndex,
            Kind = "StickyNote",
            Rect = rect,
            ColorRgba = color ?? [255, 193, 7, 255], // Amber note
            Contents = text
        };
    }

    public static AnnotationDto CreateUnderline(ulong id, uint pageIndex, PdfRect rect, byte[]? color = null)
    {
        return new AnnotationDto
        {
            Id = id,
            PageIndex = pageIndex,
            Kind = "Underline",
            Rect = rect,
            ColorRgba = color ?? [0, 122, 255, 255], // Blue underline
            Contents = string.Empty
        };
    }

    public static AnnotationDto CreateStrikeOut(ulong id, uint pageIndex, PdfRect rect, byte[]? color = null)
    {
        return new AnnotationDto
        {
            Id = id,
            PageIndex = pageIndex,
            Kind = "StrikeOut",
            Rect = rect,
            ColorRgba = color ?? [230, 38, 0, 255], // Red strikeout
            Contents = string.Empty
        };
    }

    public static AnnotationDto CreateFreehandInk(ulong id, uint pageIndex, PdfRect boundingBox, List<(float X, float Y)> points, float strokeWidth = 2.0f, byte[]? color = null)
    {
        var rawCoords = new List<float[]>();
        foreach (var (x, y) in points)
        {
            rawCoords.Add([x, y]);
        }

        var kindObj = new Dictionary<string, object>
        {
            ["FreehandInk"] = new Dictionary<string, object>
            {
                ["points"] = rawCoords,
                ["stroke_width"] = strokeWidth
            }
        };

        return new AnnotationDto
        {
            Id = id,
            PageIndex = pageIndex,
            Kind = kindObj,
            Rect = boundingBox,
            Points = points,
            StrokeWidth = strokeWidth,
            ColorRgba = color ?? [0, 0, 0, 255], // Black ink
            Contents = string.Empty
        };
    }

    public static AnnotationDto CreateFreeText(ulong id, uint pageIndex, PdfRect rect, string text, float fontSize = 12.0f, byte[]? color = null)
    {
        var kindObj = new Dictionary<string, object>
        {
            ["FreeText"] = new Dictionary<string, object>
            {
                ["font_size"] = fontSize
            }
        };

        return new AnnotationDto
        {
            Id = id,
            PageIndex = pageIndex,
            Kind = kindObj,
            Rect = rect,
            FontSize = fontSize,
            ColorRgba = color ?? [0, 0, 0, 255],
            Contents = text
        };
    }
}
