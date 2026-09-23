module InferenceTests

open NUnit.Framework
open System.Xml.Schema
open FSharp.Data
open FSharp.Data.Runtime
open FSharp.Data.Runtime.BaseTypes
open FSharp.Data.Runtime.StructuralTypes
open ProviderImplementation

let describeSchema (name: string) =
    if name.Length > 0 then name else "unknown"
