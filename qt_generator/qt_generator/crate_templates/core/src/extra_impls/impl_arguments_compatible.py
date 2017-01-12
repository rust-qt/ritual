
def types_list(nums):
  r = ", ".join("T%d" % x for x in nums)
  if len(nums) == 1:
    r += ","
  return r

for i in range(0, 17):
  for j in range(0, i + 1):
    all_ts = types_list(range(1, i+1))
    arg_ts = types_list(range(1, j+1))
    print "impl<%(all_ts)s> ArgumentsCompatible<(%(arg_ts)s)> for (%(all_ts)s) {}" % { 'all_ts': all_ts, 'arg_ts': arg_ts}
  print ""
