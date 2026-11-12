package desk

import "errors"

var errNotYet = errors.New("まだ作っていない")

func cmdCheck(e *env, args []string) error { return errNotYet }
