# Welcome to Rat-Lang language

```
________________________________________
|               |\__/,|   (`\          |
|             _.|o o  |_   ) )         |
|-------------(((---(((----------------|
|      ___          ___                |
|     /  /\        /  /\        ___    |
|    /  /:/       /  /::\      /  /\   |
|   /  /:/       /  /:/\:\    /  /:/   |
|  /  /:/  ___  /  /:/~/::\  /  /:/    |
| /__/:/  /  /\/__/:/ /:/\:\/  /::\    |
| \  \:\ /  /:/\  \:\/:/__\/__/:/\:\   |
|  \  \:\  /:/  \  \::/    \__\/  \:\  |
|   \  \:\/:/    \  \:\         \  \:\ |
|    \  \::/      \  \:\         \__\/ |
|     \__\/        \__\/               |
|______________________________________|
```

## Info

This project does a  compiler using LLVM in Rust.

...

## syntax

### variable

```python
let a:int = 1;
let a:double = 1.1;
let:str = "hello, catlang!"
// local letiable
{
    let a:int=2;
    ...
}
```
### type inference

```python
let a = 1;
let b = "aaa";

```


### control flow

```python
if (true)
{
    print("True");
}
else
{
    print("false");
}
```

### loop

only support while

developing for loop in python style...

```python
while(cond){
    statement...;
}
```

### function

closure? overload?

```python
def add(x:int, y:int)->int
{
    return x+y;
}
```

### list


```python
let l:list<int> = [1, 2, 3];
let s:int = l[0];

let l2:list<list<int>> = [[1, 2, 3], [4, 5, 6]];
let s2:list<int> = l[0];
let t2:int = s[0];

# l2[0][0] is not supported
```

### class

support "**call**" magic method and ploymorphism.

more magic methods developping?

```python
class Point {
    let x:int;
    let y:int;
}

impl Point {
    // 获取 x 坐标
    def get_x(self:Point)->int {
        return self.x;
    }
    // 获取 y 坐标
    def get_y(self:Point)->int {
        return self.y;
    }
    // 向量加法，返回新 Point
    def add(self:Point, other:Point)->Point {
        return Point(self.x + other.x, self.y + other.y);
    }
    // 向量缩放，返回新 Point
    def scale(self:Point, factor:int)->Point {
        return Point(self.x * factor, self.y * factor);
    }
    // 计算到另一个点的距离平方（避免浮点数）
    def distance_sq(self:Point, other:Point)->int {
        let dx:int = self.x - other.x;
        let dy:int = self.y - other.y;
        return dx * dx + dy * dy;
    }
}

class Circle {
    let center:Point;
    let radius:int;
}

impl Circle {
    // 计算圆面积（π 近似为 3）
    def area(self:Circle)->int {
        return self.radius * self.radius * 3;
    }
    // 判断点是否在圆内（返回 1 表示在内部，0 表示外部）
    def contains(self:Circle, p:Point)->int {
        let dist_sq:int = self.center.distance_sq(p);
        let r_sq:int = self.radius * self.radius;
        if dist_sq < r_sq {
            return 1;
        } else {
            return 0;
        }
    }
}

def main()->int {
    // 创建两个点
    let p1: Point = Point(1, 2);
    let p2: Point = Point(4, 6);

    // 点的加法与缩放
    let p3: Point = p1.add(p2);               // (5, 8)
    let p4: Point = p3.scale(2);              // (10, 16)

    // 创建圆，圆心为 p4，半径为 5
    let c: Circle = Circle(p4, 5);
    let area: int = c.area();                 // 5*5*3 = 75

    // 判断点 (10,10) 是否在圆内
    let inside: int = c.contains(Point(10, 10));

    // 循环求和：0 到 9 的和
    let sum: int = 0;
    let i: int = 0;
    while i < 10 {
        sum = sum + i;
        i = i + 1;
    }   // sum = 45

    // 链式调用：p1 加 p2 后再缩放 3 倍
    let p5: Point = p1.add(p2).scale(3);      // (15, 24)

    // 组合多个结果，作为主函数返回值
    return area + inside + sum + p5.get_x() + p5.get_y();
    // 75 + inside(0) + 45 + 15 + 24 = 159
}
`
