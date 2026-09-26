module HtmlRuntimeTypes

open NUnit.Framework
open FsUnit
open System
open System.Reflection
open FSharp.Data
open FSharp.Data.Runtime
open FSharp.Data.Runtime.BaseTypes

let buildHtml (name: string) =
    sprintf "<main>%s</main>" name
