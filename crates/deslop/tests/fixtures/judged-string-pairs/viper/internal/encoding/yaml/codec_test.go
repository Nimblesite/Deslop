package yaml

const original = `# key-value pair
key: value
list:
    - item1
    - item2
    - item3
map:
    key: value

# nested
# map
nested_map:
    map:
        key: value
        list:
            - item1
            - item2
            - item3
`

// encoded form of the data.
const encoded = `key: value
list:
    - item1
    - item2
    - item3
map:
    key: value
nested_map:
    map:
        key: value
        list:
            - item1
            - item2
            - item3
`

// decoded form of the data.
//
// In case of YAML it's slightly different from Viper's internal representation
// (e.g. map is decoded into a map with interface key).
var decoded = map[string]any{
	"key": "value",
	"list": []any{
		"item1",
		"item2",
		"item3",
	},
	"map": map[string]any{
		"key": "value",
	},
	"nested_map": map[string]any{
		"map": map[string]any{
			"key": "value",
			"list": []any{
				"item1",
				"item2",
				"item3",
			},
		},
	},
}

var data = map[string]any{
	"key": "value",
	"list": []any{
		"item1",
		"item2",
		"item3",
	},
	"map": map[string]any{
		"key": "value",
	},
	"nested_map": map[string]any{
		"map": map[string]any{
			"key": "value",
			"list": []any{
				"item1",
				"item2",
				"item3",
			},
		},
	},
}
