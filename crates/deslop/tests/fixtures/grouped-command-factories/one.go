// [FUSED-CONTENT-GATE-CALL-TARGET-PADDING] The command factories judged CLEARLY IN in corpus/register/cobra.json.
package commands
func TestCompletionForOneRequiredGroupFlags(t *testing.T) {
	getCmd := func() *Command {
		rootCmd := &Command{
			Use: "root",
			Run: emptyRun,
		}
		childCmd := &Command{
			Use: "child",
			ValidArgsFunction: func(cmd *Command, args []string, toComplete string) ([]string, ShellCompDirective) {
				return []string{"subArg"}, ShellCompDirectiveNoFileComp
			},
			Run: emptyRun,
		}
		rootCmd.AddCommand(childCmd)

		rootCmd.PersistentFlags().Int("ingroup1", -1, "ingroup1")
		rootCmd.PersistentFlags().String("ingroup2", "", "ingroup2")

		childCmd.Flags().Bool("ingroup3", false, "ingroup3")
		childCmd.Flags().Bool("nogroup", false, "nogroup")

		// Add flags to a group
		childCmd.MarkFlagsOneRequired("ingroup1", "ingroup2", "ingroup3")

		return rootCmd
	}

	// Each test case uses a unique command from the function above.
	testcases := []struct {
		desc           string
		args           []string
		expectedOutput string
	}{
		{
			desc: "flags in group suggested without - prefix",
			args: []string{"child", ""},
			expectedOutput: strings.Join([]string{
				"--ingroup1",
				"--ingroup2",
				"--ingroup3",
				"subArg",
				":4",
				"Completion ended with directive: ShellCompDirectiveNoFileComp", ""}, "\n"),
		},
		{
			desc: "flags in group suggested with - prefix",
			args: []string{"child", "-"},
			expectedOutput: strings.Join([]string{
				"--ingroup1",
				"--ingroup2",
				"--ingroup3",
				":4",
				"Completion ended with directive: ShellCompDirectiveNoFileComp", ""}, "\n"),
		},
		{
			desc: "when any flag in group present, other flags in group not suggested without - prefix",
			args: []string{"child", "--ingroup2", "value", ""},
			expectedOutput: strings.Join([]string{
				"subArg",
				":4",
				"Completion ended with directive: ShellCompDirectiveNoFileComp", ""}, "\n"),
		},
		{
			desc: "when all flags in group present, flags not suggested without - prefix",
			args: []string{"child", "--ingroup1", "8", "--ingroup2", "value2", "--ingroup3", ""},
			expectedOutput: strings.Join([]string{
				"subArg",
				":4",
				"Completion ended with directive: ShellCompDirectiveNoFileComp", ""}, "\n"),
		},
		{
			desc: "group ignored if some flags not applicable",
			args: []string{"--ingroup2", "value", ""},
			expectedOutput: strings.Join([]string{
				"child",
				"completion",
				"help",
				":4",
				"Completion ended with directive: ShellCompDirectiveNoFileComp", ""}, "\n"),
		},
	}

	for _, tc := range testcases {
		t.Run(tc.desc, func(t *testing.T) {
			c := getCmd()
			args := []string{ShellCompNoDescRequestCmd}
			args = append(args, tc.args...)
			output, err := executeCommand(c, args...)
			switch {
			case err == nil && output != tc.expectedOutput:
				t.Errorf("expected: %q, got: %q", tc.expectedOutput, output)
			case err != nil:
				t.Errorf("Unexpected error %q", err)
			}
		})
	}
}
