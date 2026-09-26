namespace Fixture

type HtmlExtensions =
    [<Extension>]
    static member Descendants(n: HtmlNode, predicate, recurseOnMatch) =
        HtmlNode.descendants recurseOnMatch predicate n

    [<Extension>]
    static member DescendantsAndSelf(n: HtmlNode, predicate, recurseOnMatch) =
        HtmlNode.descendantsAndSelf recurseOnMatch predicate n

    [<Extension>]
    static member Descendants(n: HtmlNode, predicate) =
        let recurseOnMatch = true
        HtmlNode.descendants recurseOnMatch predicate n

    [<Extension>]
    static member DescendantsAndSelf(n: HtmlNode, predicate) =
        let recurseOnMatch = true
        HtmlNode.descendantsAndSelf recurseOnMatch predicate n

    [<Extension>]
    static member Descendants(n: HtmlNode) =
        let recurseOnMatch = true
        let predicate = fun _ -> true
        HtmlNode.descendants recurseOnMatch predicate n

    [<Extension>]
    static member DescendantsAndSelf(n: HtmlNode) =
        let recurseOnMatch = true
        let predicate = fun _ -> true
        HtmlNode.descendantsAndSelf recurseOnMatch predicate n

    [<Extension>]
    static member Descendants(n: HtmlNode, names: seq<string>, recurseOnMatch) =
        HtmlNode.descendantsNamed recurseOnMatch names n

    [<Extension>]
    static member DescendantsAndSelf(n: HtmlNode, names: seq<string>, recurseOnMatch) =
        HtmlNode.descendantsAndSelfNamed recurseOnMatch names n

    [<Extension>]
    static member Descendants(n: HtmlNode, names: seq<string>) =
        let recurseOnMatch = true
        HtmlNode.descendantsNamed recurseOnMatch names n

    [<Extension>]
    static member DescendantsAndSelf(n: HtmlNode, names: seq<string>) =
        let recurseOnMatch = true
        HtmlNode.descendantsAndSelfNamed recurseOnMatch names n

    [<Extension>]
    static member DescendantsWithPath(n: HtmlNode, predicate, recurseOnMatch) =
        HtmlNode.descendantsWithPath recurseOnMatch predicate n

    [<Extension>]
    static member DescendantsAndSelfWithPath(n: HtmlNode, predicate, recurseOnMatch) =
        HtmlNode.descendantsAndSelfWithPath recurseOnMatch predicate n

    [<Extension>]
    static member DescendantsWithPath(n: HtmlNode, predicate) =
        let recurseOnMatch = true
        HtmlNode.descendantsWithPath recurseOnMatch predicate n

    [<Extension>]
    static member DescendantsAndSelfWithPath(n: HtmlNode, predicate) =
        let recurseOnMatch = true
        HtmlNode.descendantsAndSelfWithPath recurseOnMatch predicate n

    [<Extension>]
    static member DescendantsWithPath(n: HtmlNode) =
        let recurseOnMatch = true
        let predicate = fun _ -> true
        HtmlNode.descendantsWithPath recurseOnMatch predicate n

    [<Extension>]
    static member DescendantsAndSelfWithPath(n: HtmlNode) =
        let recurseOnMatch = true
        let predicate = fun _ -> true
        HtmlNode.descendantsAndSelfWithPath recurseOnMatch predicate n

    [<Extension>]
    static member DescendantsWithPath(n: HtmlNode, names: seq<string>, recurseOnMatch) =
        HtmlNode.descendantsNamedWithPath recurseOnMatch names n

    [<Extension>]
    static member DescendantsAndSelfWithPath(n: HtmlNode, names: seq<string>, recurseOnMatch) =
        HtmlNode.descendantsAndSelfNamedWithPath recurseOnMatch names n

    [<Extension>]
    static member DescendantsWithPath(n: HtmlNode, names: seq<string>) =
        let recurseOnMatch = true
        HtmlNode.descendantsNamedWithPath recurseOnMatch names n

    [<Extension>]
    static member DescendantsAndSelfWithPath(n: HtmlNode, names: seq<string>) =
        let recurseOnMatch = true
        HtmlNode.descendantsAndSelfNamedWithPath recurseOnMatch names n
