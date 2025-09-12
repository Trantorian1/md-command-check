```bash
echo "abcdefghijklmnopqrstuvwxyz1234567890abcdefghijklmnopqrstuvwxyz1234567890abcdefghijklmnopqrstuvwxyz1234567890abcdefghijklmnopqrstuvwxyz1234567890"; echo one 1>&2
sleep 1
echo two; echo two 1>&2
sleep 1
echo three; echo three 1>&2
```


```bash
echo one
sleep 1
echo two
sleep 1
echo three
```

```bash
exit 1
```
