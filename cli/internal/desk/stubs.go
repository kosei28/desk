package desk

import "errors"

var errNotYet = errors.New("まだ作っていない")

func cmdLs(e *env, args []string) error      { return errNotYet }
func cmdGraph(e *env, args []string) error   { return errNotYet }
func cmdCheck(e *env, args []string) error   { return errNotYet }
func cmdArchive(e *env, args []string) error { return errNotYet }
func cmdMv(e *env, args []string) error      { return errNotYet }
