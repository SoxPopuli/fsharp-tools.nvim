module Option =
    let expect msg =
        function
        | Some x -> x
        | None -> failwith msg

let print_help () =
    let join (sep: string) (s: string seq) =
        let mutable res = ""
        s |> Seq.iter (fun str -> res <- res + (str + sep))

        res.Substring(0, res.Length - sep.Length)

    let msg =
        [ "ast-tool [arguments]"
          "-p --project [path]                                Project path to load" 
          "-h --help                                          Print help message"
        ]
        |> join "\n        "

    printfn "%s" msg

module Args =
    type t = { project_path: string }

    module Builder =
        type builder = { project_path: string option }
        let empty = { project_path = None }

        let build (builder: builder) : t =
            { project_path = builder.project_path |> Option.expect "Missing project path" }

    let rec private parse_arg (args: Builder.builder) (rest: string list) =
        match rest with
        | ("-h" | "--help") :: _ ->
            print_help ()
            exit 0
        | ("-p" | "--project") :: proj :: rest -> parse_arg { args with project_path = Some proj } rest
        | _ -> Builder.build args


    let parse (args: string array) =
        args |> List.ofArray |> parse_arg Builder.empty

[<EntryPoint>]
let main args =
    let args = Args.parse args

    printfn "%A" args
    0
