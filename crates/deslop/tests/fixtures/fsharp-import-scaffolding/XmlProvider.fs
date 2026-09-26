namespace ProviderImplementation

open System.IO
open System.Xml.Linq
open System.Xml.Schema
open FSharp.Core.CompilerServices
open ProviderImplementation
open ProviderImplementation.ProvidedTypes
open ProviderImplementation.ProviderHelpers
open FSharp.Data
open FSharp.Data.Runtime
open FSharp.Data.Runtime.BaseTypes
open FSharp.Data.Runtime.StructuralTypes
open FSharp.Data.Runtime.StructuralInference
open System.Net

#nowarn "10001"

let renderXml (value: string) =
    value.Split(':') |> Array.length
