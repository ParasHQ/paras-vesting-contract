from datetime import datetime
from datetime import timezone
oct_1 = 1633046400000000000
one_month = 2629746000000000
one_month = 30*24*60*60*10**9

for i in range(20):
    t = oct_1 + one_month*i
    dt = datetime.fromtimestamp(t // 10**9, tz=timezone.utc)
    s = dt.strftime('%m-%d-%Y %H:%M:%S')
    print(s)

